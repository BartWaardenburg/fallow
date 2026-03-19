use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::visitor::ModuleInfoExtractor;
use super::ModuleInfo;
use crate::discover::FileId;

/// Extract import/export statements from MDX content.
///
/// MDX files are Markdown with JSX. Only `import` and `export` lines are relevant
/// for dead code analysis. Multi-line imports (with unmatched braces) are handled
/// by tracking brace depth.
///
/// NOTE: CSS/SCSS `@apply` is handled in `parse_css_to_module()`, not here.
/// MDX import/export extraction only handles JS/TS `import`/`export` statements.
pub(crate) fn extract_mdx_statements(source: &str) -> String {
    let mut statements = Vec::new();
    let mut in_multiline = false;
    let mut brace_depth: i32 = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if in_multiline {
            statements.push(line.to_string());
            brace_depth += trimmed.chars().filter(|&c| c == '{').count() as i32;
            brace_depth -= trimmed.chars().filter(|&c| c == '}').count() as i32;
            if brace_depth <= 0
                || trimmed.ends_with(';')
                || trimmed.contains(" from ")
                || trimmed.contains(" from'")
                || trimmed.contains(" from\"")
            {
                in_multiline = false;
                brace_depth = 0;
            }
        } else if trimmed.starts_with("import ")
            || trimmed.starts_with("import{")
            || trimmed.starts_with("export ")
            || trimmed.starts_with("export{")
        {
            statements.push(line.to_string());
            brace_depth = trimmed.chars().filter(|&c| c == '{').count() as i32
                - trimmed.chars().filter(|&c| c == '}').count() as i32;
            if brace_depth > 0 && !trimmed.contains(" from ") {
                in_multiline = true;
            }
        }
    }

    statements.join("\n")
}

pub(super) fn is_mdx_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "mdx")
}

/// Parse an MDX file by extracting import/export statements.
pub(super) fn parse_mdx_to_module(file_id: FileId, source: &str, content_hash: u64) -> ModuleInfo {
    let suppressions = crate::suppress::parse_suppressions_from_source(source);
    let statements = extract_mdx_statements(source);

    if !statements.is_empty() {
        let source_type = SourceType::jsx();
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &statements, source_type).parse();
        let mut extractor = ModuleInfoExtractor::new();
        extractor.visit_program(&parser_return.program);
        return extractor.into_module_info(file_id, content_hash, suppressions);
    }

    ModuleInfoExtractor::new().into_module_info(file_id, content_hash, suppressions)
}
