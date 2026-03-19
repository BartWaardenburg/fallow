use std::path::Path;
use std::sync::LazyLock;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::sfc::SfcScript;
use super::visitor::ModuleInfoExtractor;
use super::ModuleInfo;
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
