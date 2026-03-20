//! CSS/SCSS file parsing and CSS Module class name extraction.
//!
//! Handles `@import`, `@use`, `@forward`, `@apply`, `@tailwind` directives,
//! and extracts class names as named exports from `.module.css`/`.module.scss` files.

use std::path::Path;
use std::sync::LazyLock;

use oxc_span::Span;

use crate::{ExportInfo, ExportName, ImportInfo, ImportedName, ModuleInfo};
use fallow_types::discover::FileId;

/// Regex to extract CSS @import sources.
/// Matches: @import "path"; @import 'path'; @import url("path"); @import url('path'); @import url(path);
static CSS_IMPORT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"@import\s+(?:url\(\s*(?:["']([^"']+)["']|([^)]+))\s*\)|["']([^"']+)["'])"#)
        .expect("valid regex")
});

/// Regex to extract SCSS @use and @forward sources.
/// Matches: @use "path"; @use 'path'; @forward "path"; @forward 'path';
static SCSS_USE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"@(?:use|forward)\s+["']([^"']+)["']"#).expect("valid regex")
});

/// Regex to extract @apply class references.
/// Matches: @apply class1 class2 class3;
static CSS_APPLY_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"@apply\s+[^;}\n]+").expect("valid regex"));

/// Regex to extract @tailwind directives.
/// Matches: @tailwind base; @tailwind components; @tailwind utilities;
static CSS_TAILWIND_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"@tailwind\s+\w+").expect("valid regex"));

/// Regex to match CSS block comments (`/* ... */`) for stripping before extraction.
static CSS_COMMENT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)/\*.*?\*/").expect("valid regex"));

/// Regex to match SCSS single-line comments (`// ...`) for stripping before extraction.
static SCSS_LINE_COMMENT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"//[^\n]*").expect("valid regex"));

/// Regex to extract CSS class names from selectors.
/// Matches `.className` in selectors. Applied after stripping comments, strings, and URLs.
static CSS_CLASS_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\.([a-zA-Z_][a-zA-Z0-9_-]*)").expect("valid regex"));

/// Regex to strip quoted strings and `url(...)` content from CSS before class extraction.
/// Prevents false positives from `content: ".foo"` and `url(./path/file.ext)`.
static CSS_NON_SELECTOR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?s)"[^"]*"|'[^']*'|url\([^)]*\)"#).expect("valid regex")
});

pub(crate) fn is_css_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "css" || ext == "scss")
}

fn is_css_module_file(path: &Path) -> bool {
    is_css_file(path)
        && path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|stem| stem.ends_with(".module"))
}

/// Returns true if a CSS import source is a remote URL or data URI that should be skipped.
fn is_css_url_import(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://") || source.starts_with("data:")
}

/// Normalize a CSS/SCSS import path to use `./` prefix for relative paths.
/// CSS/SCSS resolve imports without `./` prefix as relative by default,
/// unlike JS where unprefixed specifiers are bare (npm) specifiers.
/// Only applies to paths with CSS/SCSS extensions — extensionless imports
/// like `@import "tailwindcss"` are actual npm package imports.
fn normalize_css_import_path(path: String) -> String {
    if path.starts_with('.') || path.starts_with('/') || path.contains("://") {
        return path;
    }
    // Paths with CSS/SCSS extensions are relative file imports
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str());
    match ext {
        Some(e)
            if e.eq_ignore_ascii_case("css")
                || e.eq_ignore_ascii_case("scss")
                || e.eq_ignore_ascii_case("sass")
                || e.eq_ignore_ascii_case("less") =>
        {
            format!("./{path}")
        }
        _ => path,
    }
}

/// Strip comments from CSS/SCSS source to avoid matching directives inside comments.
fn strip_css_comments(source: &str, is_scss: bool) -> String {
    let stripped = CSS_COMMENT_RE.replace_all(source, "");
    if is_scss {
        SCSS_LINE_COMMENT_RE.replace_all(&stripped, "").into_owned()
    } else {
        stripped.into_owned()
    }
}

/// Extract class names from a CSS module file as named exports.
pub fn extract_css_module_exports(source: &str) -> Vec<ExportInfo> {
    let cleaned = CSS_NON_SELECTOR_RE.replace_all(source, "");
    let mut seen = rustc_hash::FxHashSet::default();
    let mut exports = Vec::new();
    for cap in CSS_CLASS_RE.captures_iter(&cleaned) {
        if let Some(m) = cap.get(1) {
            let class_name = m.as_str().to_string();
            if seen.insert(class_name.clone()) {
                exports.push(ExportInfo {
                    name: ExportName::Named(class_name),
                    local_name: None,
                    is_type_only: false,
                    span: Span::default(),
                    members: Vec::new(),
                });
            }
        }
    }
    exports
}

/// Parse a CSS/SCSS file, extracting @import, @use, @forward, @apply, and @tailwind directives.
pub(crate) fn parse_css_to_module(
    file_id: FileId,
    path: &Path,
    source: &str,
    content_hash: u64,
) -> ModuleInfo {
    let suppressions = crate::suppress::parse_suppressions_from_source(source);
    let is_scss = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "scss");

    // Strip comments before matching to avoid false positives from commented-out code.
    let stripped = strip_css_comments(source, is_scss);

    let mut imports = Vec::new();

    // Extract @import statements
    for cap in CSS_IMPORT_RE.captures_iter(&stripped) {
        let source_path = cap
            .get(1)
            .or_else(|| cap.get(2))
            .or_else(|| cap.get(3))
            .map(|m| m.as_str().trim().to_string());
        if let Some(src) = source_path
            && !src.is_empty()
            && !is_css_url_import(&src)
        {
            // CSS/SCSS @import resolves relative paths without ./ prefix,
            // so normalize to ./ to avoid bare-specifier misclassification
            let src = normalize_css_import_path(src);
            imports.push(ImportInfo {
                source: src,
                imported_name: ImportedName::SideEffect,
                local_name: String::new(),
                is_type_only: false,
                span: Span::default(),
            });
        }
    }

    // Extract SCSS @use/@forward statements
    if is_scss {
        for cap in SCSS_USE_RE.captures_iter(&stripped) {
            if let Some(m) = cap.get(1) {
                imports.push(ImportInfo {
                    source: normalize_css_import_path(m.as_str().to_string()),
                    imported_name: ImportedName::SideEffect,
                    local_name: String::new(),
                    is_type_only: false,
                    span: Span::default(),
                });
            }
        }
    }

    // If @apply or @tailwind directives exist, create a synthetic import to tailwindcss
    // to mark the dependency as used
    let has_apply = CSS_APPLY_RE.is_match(&stripped);
    let has_tailwind = CSS_TAILWIND_RE.is_match(&stripped);
    if has_apply || has_tailwind {
        imports.push(ImportInfo {
            source: "tailwindcss".to_string(),
            imported_name: ImportedName::SideEffect,
            local_name: String::new(),
            is_type_only: false,
            span: Span::default(),
        });
    }

    // For CSS module files, extract class names as named exports
    let exports = if is_css_module_file(path) {
        extract_css_module_exports(&stripped)
    } else {
        Vec::new()
    };

    ModuleInfo {
        file_id,
        exports,
        imports,
        re_exports: Vec::new(),
        dynamic_imports: Vec::new(),
        dynamic_import_patterns: Vec::new(),
        require_calls: Vec::new(),
        member_accesses: Vec::new(),
        whole_object_uses: Vec::new(),
        has_cjs_exports: false,
        content_hash,
        suppressions,
        line_offsets: fallow_types::extract::compute_line_offsets(source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to collect export names as strings from `extract_css_module_exports`.
    fn export_names(source: &str) -> Vec<String> {
        extract_css_module_exports(source)
            .into_iter()
            .filter_map(|e| match e.name {
                ExportName::Named(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    // ── is_css_file ──────────────────────────────────────────────

    #[test]
    fn is_css_file_css() {
        assert!(is_css_file(Path::new("styles.css")));
    }

    #[test]
    fn is_css_file_scss() {
        assert!(is_css_file(Path::new("styles.scss")));
    }

    #[test]
    fn is_css_file_rejects_js() {
        assert!(!is_css_file(Path::new("app.js")));
    }

    #[test]
    fn is_css_file_rejects_ts() {
        assert!(!is_css_file(Path::new("app.ts")));
    }

    #[test]
    fn is_css_file_rejects_less() {
        assert!(!is_css_file(Path::new("styles.less")));
    }

    #[test]
    fn is_css_file_rejects_no_extension() {
        assert!(!is_css_file(Path::new("Makefile")));
    }

    // ── is_css_module_file ───────────────────────────────────────

    #[test]
    fn is_css_module_file_module_css() {
        assert!(is_css_module_file(Path::new("Component.module.css")));
    }

    #[test]
    fn is_css_module_file_module_scss() {
        assert!(is_css_module_file(Path::new("Component.module.scss")));
    }

    #[test]
    fn is_css_module_file_rejects_plain_css() {
        assert!(!is_css_module_file(Path::new("styles.css")));
    }

    #[test]
    fn is_css_module_file_rejects_plain_scss() {
        assert!(!is_css_module_file(Path::new("styles.scss")));
    }

    #[test]
    fn is_css_module_file_rejects_module_js() {
        assert!(!is_css_module_file(Path::new("utils.module.js")));
    }

    // ── extract_css_module_exports: basic class extraction ───────

    #[test]
    fn extracts_single_class() {
        let names = export_names(".foo { color: red; }");
        assert_eq!(names, vec!["foo"]);
    }

    #[test]
    fn extracts_multiple_classes() {
        let names = export_names(".foo { } .bar { }");
        assert_eq!(names, vec!["foo", "bar"]);
    }

    #[test]
    fn extracts_nested_classes() {
        let names = export_names(".foo .bar { color: red; }");
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"bar".to_string()));
    }

    #[test]
    fn extracts_hyphenated_class() {
        let names = export_names(".my-class { }");
        assert_eq!(names, vec!["my-class"]);
    }

    #[test]
    fn extracts_camel_case_class() {
        let names = export_names(".myClass { }");
        assert_eq!(names, vec!["myClass"]);
    }

    #[test]
    fn extracts_underscore_class() {
        let names = export_names("._hidden { } .__wrapper { }");
        assert!(names.contains(&"_hidden".to_string()));
        assert!(names.contains(&"__wrapper".to_string()));
    }

    // ── Pseudo-selectors ─────────────────────────────────────────

    #[test]
    fn pseudo_selector_hover() {
        let names = export_names(".foo:hover { color: blue; }");
        assert_eq!(names, vec!["foo"]);
    }

    #[test]
    fn pseudo_selector_focus() {
        let names = export_names(".input:focus { outline: none; }");
        assert_eq!(names, vec!["input"]);
    }

    #[test]
    fn pseudo_element_before() {
        let names = export_names(".icon::before { content: ''; }");
        assert_eq!(names, vec!["icon"]);
    }

    #[test]
    fn combined_pseudo_selectors() {
        let names = export_names(".btn:hover, .btn:active, .btn:focus { }");
        // "btn" should be deduplicated
        assert_eq!(names, vec!["btn"]);
    }

    // ── Media queries ────────────────────────────────────────────

    #[test]
    fn classes_inside_media_query() {
        let names = export_names(
            "@media (max-width: 768px) { .mobile-nav { display: block; } .desktop-nav { display: none; } }",
        );
        assert!(names.contains(&"mobile-nav".to_string()));
        assert!(names.contains(&"desktop-nav".to_string()));
    }

    // ── Deduplication ────────────────────────────────────────────

    #[test]
    fn deduplicates_repeated_class() {
        let names = export_names(".btn { color: red; } .btn { font-size: 14px; }");
        assert_eq!(names.iter().filter(|n| *n == "btn").count(), 1);
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn empty_source() {
        let names = export_names("");
        assert!(names.is_empty());
    }

    #[test]
    fn no_classes() {
        let names = export_names("body { margin: 0; } * { box-sizing: border-box; }");
        assert!(names.is_empty());
    }

    #[test]
    fn ignores_classes_in_block_comments() {
        // Note: extract_css_module_exports itself does NOT strip comments;
        // comments are stripped in parse_css_to_module before calling it.
        // But CSS_NON_SELECTOR_RE strips quoted strings. Testing the
        // strip_css_comments + extract pipeline via the stripped source:
        let stripped = strip_css_comments("/* .fake { } */ .real { }", false);
        let names = export_names(&stripped);
        assert!(!names.contains(&"fake".to_string()));
        assert!(names.contains(&"real".to_string()));
    }

    #[test]
    fn ignores_classes_in_strings() {
        let names = export_names(r#".real { content: ".fake"; }"#);
        assert!(names.contains(&"real".to_string()));
        assert!(!names.contains(&"fake".to_string()));
    }

    #[test]
    fn ignores_classes_in_url() {
        let names = export_names(".real { background: url(./images/hero.png); }");
        assert!(names.contains(&"real".to_string()));
        // "png" from "hero.png" should not be extracted
        assert!(!names.contains(&"png".to_string()));
    }

    // ── strip_css_comments ───────────────────────────────────────

    #[test]
    fn strip_css_block_comment() {
        let result = strip_css_comments("/* removed */ .kept { }", false);
        assert!(!result.contains("removed"));
        assert!(result.contains(".kept"));
    }

    #[test]
    fn strip_scss_line_comment() {
        let result = strip_css_comments("// removed\n.kept { }", true);
        assert!(!result.contains("removed"));
        assert!(result.contains(".kept"));
    }

    #[test]
    fn strip_scss_preserves_css_outside_comments() {
        let source = "// line comment\n/* block comment */\n.visible { color: red; }";
        let result = strip_css_comments(source, true);
        assert!(result.contains(".visible"));
    }

    // ── is_css_url_import ────────────────────────────────────────

    #[test]
    fn url_import_http() {
        assert!(is_css_url_import("http://example.com/style.css"));
    }

    #[test]
    fn url_import_https() {
        assert!(is_css_url_import("https://fonts.googleapis.com/css"));
    }

    #[test]
    fn url_import_data() {
        assert!(is_css_url_import("data:text/css;base64,abc"));
    }

    #[test]
    fn url_import_local_not_skipped() {
        assert!(!is_css_url_import("./local.css"));
    }

    #[test]
    fn url_import_bare_specifier_not_skipped() {
        assert!(!is_css_url_import("tailwindcss"));
    }
}
