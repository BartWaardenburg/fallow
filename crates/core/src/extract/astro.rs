use std::path::Path;
use std::sync::LazyLock;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::ModuleInfo;
use super::sfc::SfcScript;
use super::visitor::ModuleInfoExtractor;
use crate::discover::FileId;

/// Regex to extract Astro frontmatter (content between `---` delimiters at file start).
static ASTRO_FRONTMATTER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?s)\A\s*---[ \t]*\n(?P<body>.*?\n)---").expect("valid regex")
});

/// Extract frontmatter from an Astro component.
pub(crate) fn extract_astro_frontmatter(source: &str) -> Option<SfcScript> {
    ASTRO_FRONTMATTER_RE.captures(source).map(|cap| {
        let body_match = cap.name("body");
        SfcScript {
            body: body_match.map(|m| m.as_str()).unwrap_or("").to_string(),
            is_typescript: true, // Astro frontmatter is always TS-compatible
            is_jsx: false,
            byte_offset: body_match.map(|m| m.start()).unwrap_or(0),
            src: None,
        }
    })
}

pub(super) fn is_astro_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "astro")
}

/// Parse an Astro file by extracting the frontmatter section.
pub(super) fn parse_astro_to_module(
    file_id: FileId,
    source: &str,
    content_hash: u64,
) -> ModuleInfo {
    let suppressions = crate::suppress::parse_suppressions_from_source(source);

    if let Some(script) = extract_astro_frontmatter(source) {
        let source_type = SourceType::ts();
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &script.body, source_type).parse();
        let mut extractor = ModuleInfoExtractor::new();
        extractor.visit_program(&parser_return.program);
        return extractor.into_module_info(file_id, content_hash, suppressions);
    }

    ModuleInfoExtractor::new().into_module_info(file_id, content_hash, suppressions)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_astro_file ────────────────────────────────────────────

    #[test]
    fn is_astro_file_positive() {
        assert!(is_astro_file(Path::new("Layout.astro")));
    }

    #[test]
    fn is_astro_file_rejects_vue() {
        assert!(!is_astro_file(Path::new("App.vue")));
    }

    #[test]
    fn is_astro_file_rejects_ts() {
        assert!(!is_astro_file(Path::new("utils.ts")));
    }

    #[test]
    fn is_astro_file_rejects_mdx() {
        assert!(!is_astro_file(Path::new("post.mdx")));
    }

    // ── extract_astro_frontmatter: basic extraction ──────────────

    #[test]
    fn extracts_frontmatter_body() {
        let source = "---\nimport Layout from '../layouts/Layout.astro';\nconst title = 'Hi';\n---\n<Layout />";
        let script = extract_astro_frontmatter(source);
        assert!(script.is_some());
        let script = script.unwrap();
        assert!(script.body.contains("import Layout"));
        assert!(script.body.contains("const title"));
    }

    #[test]
    fn frontmatter_is_always_typescript() {
        let source = "---\nconst x = 1;\n---\n<div />";
        let script = extract_astro_frontmatter(source).unwrap();
        assert!(script.is_typescript);
    }

    #[test]
    fn frontmatter_is_not_jsx() {
        let source = "---\nconst x = 1;\n---\n<div />";
        let script = extract_astro_frontmatter(source).unwrap();
        assert!(!script.is_jsx);
    }

    #[test]
    fn frontmatter_has_no_src() {
        let source = "---\nconst x = 1;\n---\n<div />";
        let script = extract_astro_frontmatter(source).unwrap();
        assert!(script.src.is_none());
    }

    // ── No frontmatter ───────────────────────────────────────────

    #[test]
    fn no_frontmatter_returns_none() {
        let source = "<div>No frontmatter here</div>";
        assert!(extract_astro_frontmatter(source).is_none());
    }

    #[test]
    fn no_frontmatter_just_html() {
        let source = "<html><body><h1>Hello</h1></body></html>";
        assert!(extract_astro_frontmatter(source).is_none());
    }

    // ── Empty frontmatter ────────────────────────────────────────

    #[test]
    fn empty_frontmatter() {
        let source = "---\n\n---\n<div />";
        let script = extract_astro_frontmatter(source);
        assert!(script.is_some());
        let body = script.unwrap().body;
        assert!(body.trim().is_empty());
    }

    // ── Multiple --- pairs: only first is extracted ──────────────

    #[test]
    fn only_first_frontmatter_pair() {
        let source = "---\nconst first = true;\n---\n<div />\n---\nconst second = true;\n---\n";
        let script = extract_astro_frontmatter(source);
        assert!(script.is_some());
        let body = script.unwrap().body;
        assert!(body.contains("first"));
        assert!(!body.contains("second"));
    }

    // ── Byte offset ──────────────────────────────────────────────

    #[test]
    fn byte_offset_points_to_body() {
        let source = "---\nconst x = 1;\n---\n<div />";
        let script = extract_astro_frontmatter(source).unwrap();
        let offset = script.byte_offset;
        assert!(source[offset..].starts_with("const x = 1;"));
    }

    // ── Leading whitespace before --- ────────────────────────────

    #[test]
    fn leading_whitespace_before_frontmatter() {
        let source = "  \n---\nconst x = 1;\n---\n<div />";
        let script = extract_astro_frontmatter(source);
        assert!(script.is_some());
        assert!(script.unwrap().body.contains("const x = 1;"));
    }

    // ── Frontmatter with TypeScript syntax ───────────────────────

    #[test]
    fn frontmatter_with_type_annotations() {
        let source = "---\ninterface Props { title: string; }\nconst { title } = Astro.props as Props;\n---\n<h1>{title}</h1>";
        let script = extract_astro_frontmatter(source);
        assert!(script.is_some());
        let body = script.unwrap().body;
        assert!(body.contains("interface Props"));
        assert!(body.contains("Astro.props"));
    }
}
