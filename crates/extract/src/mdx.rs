use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::ModuleInfo;
use crate::visitor::ModuleInfoExtractor;
use fallow_types::discover::FileId;

/// Extract import/export statements from MDX content.
///
/// MDX files are Markdown with JSX. Only `import` and `export` lines are relevant
/// for dead code analysis. Multi-line imports (with unmatched braces) are handled
/// by tracking brace depth.
///
/// NOTE: CSS/SCSS `@apply` is handled in `parse_css_to_module()`, not here.
/// MDX import/export extraction only handles JS/TS `import`/`export` statements.
pub fn extract_mdx_statements(source: &str) -> String {
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

pub(crate) fn is_mdx_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "mdx")
}

/// Parse an MDX file by extracting import/export statements.
pub(crate) fn parse_mdx_to_module(file_id: FileId, source: &str, content_hash: u64) -> ModuleInfo {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_mdx_file ──────────────────────────────────────────────

    #[test]
    fn is_mdx_file_positive() {
        assert!(is_mdx_file(Path::new("post.mdx")));
    }

    #[test]
    fn is_mdx_file_rejects_md() {
        assert!(!is_mdx_file(Path::new("readme.md")));
    }

    #[test]
    fn is_mdx_file_rejects_tsx() {
        assert!(!is_mdx_file(Path::new("component.tsx")));
    }

    #[test]
    fn is_mdx_file_rejects_jsx() {
        assert!(!is_mdx_file(Path::new("component.jsx")));
    }

    // ── extract_mdx_statements: import extraction ────────────────

    #[test]
    fn extracts_single_import() {
        let result = extract_mdx_statements("import { Chart } from './Chart'\n\n# Title\n");
        assert!(result.contains("import { Chart } from './Chart'"));
    }

    #[test]
    fn extracts_default_import() {
        let result = extract_mdx_statements("import Button from './Button'\n\n# Title\n");
        assert!(result.contains("import Button from './Button'"));
    }

    #[test]
    fn extracts_multiple_imports() {
        let source = "import { A } from './a'\nimport { B } from './b'\n\n# Title\n";
        let result = extract_mdx_statements(source);
        assert!(result.contains("import { A } from './a'"));
        assert!(result.contains("import { B } from './b'"));
    }

    #[test]
    fn extracts_import_no_space() {
        let result = extract_mdx_statements("import{ Chart } from './Chart'\n\n# Title\n");
        assert!(result.contains("import{ Chart }"));
    }

    // ── Export extraction ────────────────────────────────────────

    #[test]
    fn extracts_export_const() {
        let result = extract_mdx_statements("export const meta = { title: 'Hello' }\n\n# Title\n");
        assert!(result.contains("export const meta"));
    }

    #[test]
    fn extracts_export_no_space() {
        let result = extract_mdx_statements("export{ foo } from './foo'\n\n# Title\n");
        assert!(result.contains("export{ foo }"));
    }

    // ── Multi-line imports ───────────────────────────────────────

    #[test]
    fn multiline_import_with_braces() {
        let source =
            "import {\n  Chart,\n  Table,\n  Graph\n} from './components'\n\n# Dashboard\n";
        let result = extract_mdx_statements(source);
        assert!(result.contains("Chart"));
        assert!(result.contains("Table"));
        assert!(result.contains("Graph"));
        assert!(result.contains("from './components'"));
    }

    #[test]
    fn multiline_import_closed_by_from() {
        let source = "import {\n  Foo,\n  Bar\n} from './mod'\n\n# Content\n";
        let result = extract_mdx_statements(source);
        assert!(result.contains("Foo"));
        assert!(result.contains("Bar"));
    }

    // ── Mixed content ────────────────────────────────────────────

    #[test]
    fn imports_between_prose() {
        let source = "import { Header } from './Header'\n\n# Section 1\n\nSome content.\n\nimport { Footer } from './Footer'\n\n## Section 2\n";
        let result = extract_mdx_statements(source);
        assert!(result.contains("Header"));
        assert!(result.contains("Footer"));
    }

    #[test]
    fn prose_lines_excluded() {
        let source =
            "import { A } from './a'\n\n# Title\n\nSome **markdown** text.\n\n- List item\n";
        let result = extract_mdx_statements(source);
        assert!(!result.contains("Title"));
        assert!(!result.contains("markdown"));
        assert!(!result.contains("List item"));
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn empty_source() {
        let result = extract_mdx_statements("");
        assert!(result.is_empty());
    }

    #[test]
    fn no_imports_or_exports() {
        let result = extract_mdx_statements("# Just Markdown\n\nNo imports here.\n");
        assert!(result.is_empty());
    }

    #[test]
    fn import_like_text_not_extracted() {
        // "important" starts with "import" but doesn't match "import " or "import{"
        let result = extract_mdx_statements("This is an important note.\n");
        assert!(result.is_empty());
    }

    #[test]
    fn export_like_text_not_extracted() {
        // "exporting" doesn't match "export " or "export{"
        let result = extract_mdx_statements("We are exporting goods overseas.\n");
        assert!(result.is_empty());
    }

    #[test]
    fn side_effect_import() {
        let result = extract_mdx_statements("import './global.css'\n\n# Title\n");
        assert!(result.contains("import './global.css'"));
    }

    #[test]
    fn namespace_import() {
        let result = extract_mdx_statements("import * as utils from './utils'\n\n# Title\n");
        assert!(result.contains("import * as utils from './utils'"));
    }

    #[test]
    fn single_line_import_with_braces_balanced() {
        // Braces balanced on one line — should NOT enter multiline mode
        let source = "import { A } from './a'\n# Title\n";
        let result = extract_mdx_statements(source);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 1);
    }
}
