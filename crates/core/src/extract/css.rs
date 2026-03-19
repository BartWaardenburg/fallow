use std::path::Path;
use std::sync::LazyLock;

use oxc_span::Span;

use super::{ExportInfo, ExportName, ImportInfo, ImportedName, ModuleInfo};
use crate::discover::FileId;

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
    LazyLock::new(|| regex::Regex::new(r#"@apply\s+[^;}\n]+"#).expect("valid regex"));

/// Regex to extract @tailwind directives.
/// Matches: @tailwind base; @tailwind components; @tailwind utilities;
static CSS_TAILWIND_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"@tailwind\s+\w+"#).expect("valid regex"));

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

pub(super) fn is_css_file(path: &Path) -> bool {
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
pub(super) fn extract_css_module_exports(source: &str) -> Vec<ExportInfo> {
    let cleaned = CSS_NON_SELECTOR_RE.replace_all(source, "");
    let mut seen = std::collections::HashSet::new();
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
pub(super) fn parse_css_to_module(
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
                    source: m.as_str().to_string(),
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
    }
}
