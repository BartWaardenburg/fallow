use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::ModuleInfo;
use crate::astro::{is_astro_file, parse_astro_to_module};
use crate::css::{is_css_file, parse_css_to_module};
use crate::mdx::{is_mdx_file, parse_mdx_to_module};
use crate::sfc::{is_sfc_file, parse_sfc_to_module};
use crate::visitor::ModuleInfoExtractor;
use fallow_types::discover::FileId;
use fallow_types::extract::ImportInfo;

/// Parse source text into a [`ModuleInfo`].
pub fn parse_source_to_module(
    file_id: FileId,
    path: &Path,
    source: &str,
    content_hash: u64,
) -> ModuleInfo {
    if is_sfc_file(path) {
        return parse_sfc_to_module(file_id, source, content_hash);
    }
    if is_astro_file(path) {
        return parse_astro_to_module(file_id, source, content_hash);
    }
    if is_mdx_file(path) {
        return parse_mdx_to_module(file_id, source, content_hash);
    }
    if is_css_file(path) {
        return parse_css_to_module(file_id, path, source, content_hash);
    }

    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, source, source_type).parse();

    // Parse suppression comments
    let suppressions = crate::suppress::parse_suppressions(&parser_return.program.comments, source);

    // Extract imports/exports even if there are parse errors
    let mut extractor = ModuleInfoExtractor::new();
    extractor.visit_program(&parser_return.program);

    // Detect unused import bindings via oxc_semantic scope analysis
    let mut unused_bindings =
        compute_unused_import_bindings(&parser_return.program, &extractor.imports);

    // If parsing produced very few results relative to source size (likely parse errors
    // from Flow types or JSX in .js files), retry with JSX/TSX source type as a fallback.
    let total_extracted =
        extractor.exports.len() + extractor.imports.len() + extractor.re_exports.len();
    if total_extracted == 0 && source.len() > 100 && !source_type.is_jsx() {
        let jsx_type = if source_type.is_typescript() {
            SourceType::tsx()
        } else {
            SourceType::jsx()
        };
        let allocator2 = Allocator::default();
        let retry_return = Parser::new(&allocator2, source, jsx_type).parse();
        let mut retry_extractor = ModuleInfoExtractor::new();
        retry_extractor.visit_program(&retry_return.program);
        let retry_total = retry_extractor.exports.len()
            + retry_extractor.imports.len()
            + retry_extractor.re_exports.len();
        if retry_total > total_extracted {
            unused_bindings =
                compute_unused_import_bindings(&retry_return.program, &retry_extractor.imports);
            extractor = retry_extractor;
        }
    }

    let mut info = extractor.into_module_info(file_id, content_hash, suppressions);
    info.unused_import_bindings = unused_bindings;
    info.line_offsets = fallow_types::extract::compute_line_offsets(source);
    info
}

/// Use `oxc_semantic` to find import bindings that are never referenced in the file.
///
/// An import like `import { foo } from './utils'` where `foo` is never used
/// anywhere in the file should not count as a reference to the `foo` export.
/// This improves unused-export detection precision.
///
/// Note: `get_resolved_references` counts both value-context and type-context
/// references. A value import used only as a type annotation (`const x: Foo`)
/// will have a type-position reference and will NOT appear in the unused list.
/// This is correct: `import { Foo }` (without `type`) may be needed at runtime.
fn compute_unused_import_bindings(program: &Program<'_>, imports: &[ImportInfo]) -> Vec<String> {
    use oxc_semantic::SemanticBuilder;

    // Skip files with no imports
    if imports.is_empty() {
        return Vec::new();
    }

    let semantic_ret = SemanticBuilder::new().build(program);
    let semantic = semantic_ret.semantic;
    let scoping = semantic.scoping();
    let root_scope = scoping.root_scope_id();

    let mut unused = Vec::new();
    for import in imports {
        // Side-effect imports have no binding
        if import.local_name.is_empty() {
            continue;
        }
        // Look up the import binding in the module scope
        let name = oxc_span::Ident::from(import.local_name.as_str());
        if let Some(symbol_id) = scoping.get_binding(root_scope, name)
            && scoping.get_resolved_references(symbol_id).count() == 0
        {
            unused.push(import.local_name.clone());
        }
    }
    unused
}
