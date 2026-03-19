use std::path::Path;
use std::sync::LazyLock;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::visitor::ModuleInfoExtractor;
use super::{ImportInfo, ImportedName, ModuleInfo};
use crate::discover::FileId;
use oxc_span::Span;

/// Regex to extract `<script>` block content from Vue/Svelte SFCs.
/// The attrs pattern handles `>` inside quoted attribute values (e.g., `generic="T extends Foo<Bar>"`).
static SCRIPT_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?is)<script\b(?P<attrs>(?:[^>"']|"[^"]*"|'[^']*')*)>(?P<body>[\s\S]*?)</script>"#,
    )
    .expect("valid regex")
});

/// Regex to extract the `lang` attribute value from a script tag.
static LANG_ATTR_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"lang\s*=\s*["'](\w+)["']"#).expect("valid regex"));

/// Regex to extract the `src` attribute value from a script tag.
/// Requires whitespace (or start of string) before `src` to avoid matching `data-src` etc.
static SRC_ATTR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?:^|\s)src\s*=\s*["']([^"']+)["']"#).expect("valid regex")
});

/// Regex to match HTML comments for filtering script blocks inside comments.
static HTML_COMMENT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)<!--.*?-->").expect("valid regex"));

pub(crate) struct SfcScript {
    pub body: String,
    pub is_typescript: bool,
    /// Whether the script uses JSX syntax (lang="tsx" or lang="jsx").
    pub is_jsx: bool,
    /// Byte offset of the script body within the full SFC source.
    pub byte_offset: usize,
    /// External script source path from `src` attribute.
    pub src: Option<String>,
}

pub(crate) fn extract_sfc_scripts(source: &str) -> Vec<SfcScript> {
    // Build HTML comment ranges to filter out <script> blocks inside comments.
    // Using ranges instead of source replacement avoids corrupting script body content
    // (e.g., string literals containing "<!--" would be destroyed by replacement).
    let comment_ranges: Vec<(usize, usize)> = HTML_COMMENT_RE
        .find_iter(source)
        .map(|m| (m.start(), m.end()))
        .collect();

    SCRIPT_BLOCK_RE
        .captures_iter(source)
        .filter(|cap| {
            let start = cap.get(0).map(|m| m.start()).unwrap_or(0);
            !comment_ranges
                .iter()
                .any(|&(cs, ce)| start >= cs && start < ce)
        })
        .map(|cap| {
            let attrs = cap.name("attrs").map(|m| m.as_str()).unwrap_or("");
            let body_match = cap.name("body");
            let byte_offset = body_match.map(|m| m.start()).unwrap_or(0);
            let body = body_match.map(|m| m.as_str()).unwrap_or("").to_string();
            let lang = LANG_ATTR_RE
                .captures(attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str());
            let is_typescript = matches!(lang, Some("ts" | "tsx"));
            let is_jsx = matches!(lang, Some("tsx" | "jsx"));
            let src = SRC_ATTR_RE
                .captures(attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
            SfcScript {
                body,
                is_typescript,
                is_jsx,
                byte_offset,
                src,
            }
        })
        .collect()
}

pub(crate) fn is_sfc_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "vue" || ext == "svelte")
}

/// Parse an SFC file by extracting and combining all `<script>` blocks.
pub(super) fn parse_sfc_to_module(file_id: FileId, source: &str, content_hash: u64) -> ModuleInfo {
    let scripts = extract_sfc_scripts(source);

    // For SFC files, use string scanning for suppression comments since script block
    // byte offsets don't correspond to the original file positions.
    let suppressions = crate::suppress::parse_suppressions_from_source(source);

    let mut combined = ModuleInfo {
        file_id,
        exports: Vec::new(),
        imports: Vec::new(),
        re_exports: Vec::new(),
        dynamic_imports: Vec::new(),
        dynamic_import_patterns: Vec::new(),
        require_calls: Vec::new(),
        member_accesses: Vec::new(),
        whole_object_uses: Vec::new(),
        has_cjs_exports: false,
        content_hash,
        suppressions,
    };

    for script in &scripts {
        if let Some(src) = &script.src {
            combined.imports.push(ImportInfo {
                source: src.clone(),
                imported_name: ImportedName::SideEffect,
                local_name: String::new(),
                is_type_only: false,
                span: Span::default(),
            });
        }

        let source_type = match (script.is_typescript, script.is_jsx) {
            (true, true) => SourceType::tsx(),
            (true, false) => SourceType::ts(),
            (false, true) => SourceType::jsx(),
            (false, false) => SourceType::mjs(),
        };
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &script.body, source_type).parse();
        let mut extractor = ModuleInfoExtractor::new();
        extractor.visit_program(&parser_return.program);
        extractor.merge_into(&mut combined);
    }

    combined
}
