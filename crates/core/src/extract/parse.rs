use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::astro::{is_astro_file, parse_astro_to_module};
use super::css::{is_css_file, parse_css_to_module};
use super::mdx::{is_mdx_file, parse_mdx_to_module};
use super::sfc::{is_sfc_file, parse_sfc_to_module};
use super::visitor::ModuleInfoExtractor;
use super::ModuleInfo;
use crate::discover::FileId;

/// Parse source text into a ModuleInfo.
pub(crate) fn parse_source_to_module(
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
            extractor = retry_extractor;
        }
    }

    extractor.into_module_info(file_id, content_hash, suppressions)
}
