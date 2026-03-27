//! Module extraction types: exports, imports, re-exports, members, and parse results.

use oxc_span::Span;

use crate::discover::FileId;
use crate::suppress::Suppression;

/// Extracted module information from a single file.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Unique identifier for this file.
    pub file_id: FileId,
    /// All export declarations in this module.
    pub exports: Vec<ExportInfo>,
    /// All import declarations in this module.
    pub imports: Vec<ImportInfo>,
    /// All re-export declarations (e.g., `export { foo } from './bar'`).
    pub re_exports: Vec<ReExportInfo>,
    /// All dynamic `import()` calls with string literal sources.
    pub dynamic_imports: Vec<DynamicImportInfo>,
    /// Dynamic import patterns from template literals, string concat, or `import.meta.glob`.
    pub dynamic_import_patterns: Vec<DynamicImportPattern>,
    /// All `require()` calls.
    pub require_calls: Vec<RequireCallInfo>,
    /// Static member access expressions (e.g., `Status.Active`).
    pub member_accesses: Vec<MemberAccess>,
    /// Identifiers used in "all members consumed" patterns
    /// (Object.values, Object.keys, Object.entries, Object.getOwnPropertyNames, for..in, spread, computed dynamic access).
    pub whole_object_uses: Vec<String>,
    /// Whether this module uses `CommonJS` exports (`module.exports` or `exports.*`).
    pub has_cjs_exports: bool,
    /// xxh3 hash of the file content for incremental caching.
    pub content_hash: u64,
    /// Inline suppression directives parsed from comments.
    pub suppressions: Vec<Suppression>,
    /// Local names of import bindings that are never referenced in this file.
    /// Populated via `oxc_semantic` scope analysis. Used at graph-build time
    /// to skip adding references for imports whose binding is never read,
    /// improving unused-export detection precision.
    pub unused_import_bindings: Vec<String>,
    /// Pre-computed byte offsets where each line starts, for O(log N) byte-to-line/col conversion.
    /// Entry `i` is the byte offset of the start of line `i` (0-indexed).
    /// Example: for "abc\ndef\n", `line_offsets` = \[0, 4\].
    pub line_offsets: Vec<u32>,
    /// Per-function complexity metrics computed during AST traversal.
    /// Used by the `fallow health` subcommand to report high-complexity functions.
    pub complexity: Vec<FunctionComplexity>,
}

/// Compute a table of line-start byte offsets from source text.
///
/// The returned vec contains one entry per line: `line_offsets[i]` is the byte
/// offset where line `i` starts (0-indexed). The first entry is always `0`.
#[must_use]
pub fn compute_line_offsets(source: &str) -> Vec<u32> {
    let mut offsets = vec![0u32];
    for (i, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push((i + 1) as u32);
        }
    }
    offsets
}

/// Convert a byte offset to a 1-based line number and 0-based byte column
/// using a pre-computed line offset table (from [`compute_line_offsets`]).
///
/// Uses binary search for O(log L) lookup where L is the number of lines.
#[must_use]
pub fn byte_offset_to_line_col(line_offsets: &[u32], byte_offset: u32) -> (u32, u32) {
    // Binary search: find the last line whose start is <= byte_offset
    let line_idx = match line_offsets.binary_search(&byte_offset) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    };
    let line = line_idx as u32 + 1; // 1-based
    let col = byte_offset - line_offsets[line_idx];
    (line, col)
}

/// Complexity metrics for a single function/method/arrow.
#[derive(Debug, Clone, serde::Serialize, bincode::Encode, bincode::Decode)]
pub struct FunctionComplexity {
    /// Function name (or `"<anonymous>"` for unnamed functions/arrows).
    pub name: String,
    /// 1-based line number where the function starts.
    pub line: u32,
    /// 0-based byte column where the function starts.
    pub col: u32,
    /// `McCabe` cyclomatic complexity (1 + decision points).
    pub cyclomatic: u16,
    /// `SonarSource` cognitive complexity (structural + nesting penalty).
    pub cognitive: u16,
    /// Number of lines in the function body.
    pub line_count: u32,
}

/// A dynamic import with a pattern that can be partially resolved (e.g., template literals).
#[derive(Debug, Clone)]
pub struct DynamicImportPattern {
    /// Static prefix of the import path (e.g., "./locales/"). May contain glob characters.
    pub prefix: String,
    /// Static suffix of the import path (e.g., ".json"), if any.
    pub suffix: Option<String>,
    /// Source span in the original file.
    pub span: Span,
}

/// An export declaration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportInfo {
    /// The exported name (named or default).
    pub name: ExportName,
    /// The local binding name, if different from the exported name.
    pub local_name: Option<String>,
    /// Whether this is a type-only export (`export type`).
    pub is_type_only: bool,
    /// Whether this export has a `@public` JSDoc/TSDoc tag.
    /// Exports marked `@public` are never reported as unused — they are
    /// assumed to be consumed by external consumers (library API surface).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_public: bool,
    /// Source span of the export declaration.
    #[serde(serialize_with = "serialize_span")]
    pub span: Span,
    /// Members of this export (for enums and classes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberInfo>,
}

/// A member of an enum or class.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemberInfo {
    /// Member name.
    pub name: String,
    /// Whether this is an enum member, class method, or class property.
    pub kind: MemberKind,
    /// Source span of the member declaration.
    #[serde(serialize_with = "serialize_span")]
    pub span: Span,
    /// Whether this member has decorators (e.g., `@Column()`, `@Inject()`).
    /// Decorated members are used by frameworks at runtime and should not be
    /// flagged as unused class members.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_decorator: bool,
}

/// The kind of member.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, bincode::Encode, bincode::Decode)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    /// A TypeScript enum member.
    EnumMember,
    /// A class method.
    ClassMethod,
    /// A class property.
    ClassProperty,
}

/// A static member access expression (e.g., `Status.Active`, `MyClass.create()`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub struct MemberAccess {
    /// The identifier being accessed (the import name).
    pub object: String,
    /// The member being accessed.
    pub member: String,
}

#[expect(clippy::trivially_copy_pass_by_ref)] // serde serialize_with requires &T
fn serialize_span<S: serde::Serializer>(span: &Span, serializer: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(2))?;
    map.serialize_entry("start", &span.start)?;
    map.serialize_entry("end", &span.end)?;
    map.end()
}

/// Export identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum ExportName {
    /// A named export (e.g., `export const foo`).
    Named(String),
    /// The default export.
    Default,
}

impl ExportName {
    /// Compare against a string without allocating (avoids `to_string()`).
    #[must_use]
    pub fn matches_str(&self, s: &str) -> bool {
        match self {
            Self::Named(n) => n == s,
            Self::Default => s == "default",
        }
    }
}

impl std::fmt::Display for ExportName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Named(n) => write!(f, "{n}"),
            Self::Default => write!(f, "default"),
        }
    }
}

/// An import declaration.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// The import specifier (e.g., `./utils` or `react`).
    pub source: String,
    /// How the symbol is imported (named, default, namespace, or side-effect).
    pub imported_name: ImportedName,
    /// The local binding name in the importing module.
    pub local_name: String,
    /// Whether this is a type-only import (`import type`).
    pub is_type_only: bool,
    /// Source span of the import declaration.
    pub span: Span,
    /// Span of the source string literal (e.g., the `'./utils'` in `import { foo } from './utils'`).
    /// Used by the LSP to highlight just the specifier in diagnostics.
    pub source_span: Span,
}

/// How a symbol is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportedName {
    /// A named import (e.g., `import { foo }`).
    Named(String),
    /// A default import (e.g., `import React`).
    Default,
    /// A namespace import (e.g., `import * as utils`).
    Namespace,
    /// A side-effect import (e.g., `import './styles.css'`).
    SideEffect,
}

// Size assertions to prevent memory regressions in hot-path types.
// These types are stored in Vecs inside `ModuleInfo` (one per file) and are
// iterated during graph construction and analysis. Keeping them compact
// improves cache locality on large projects with thousands of files.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ExportInfo>() == 88);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ImportInfo>() == 96);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ExportName>() == 24);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ImportedName>() == 24);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<MemberAccess>() == 48);
// `ModuleInfo` is the per-file extraction result — stored in a Vec during parallel parsing.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ModuleInfo>() == 304);

/// A re-export declaration.
#[derive(Debug, Clone)]
pub struct ReExportInfo {
    /// The module being re-exported from.
    pub source: String,
    /// The name imported from the source module (or `*` for star re-exports).
    pub imported_name: String,
    /// The name exported from this module.
    pub exported_name: String,
    /// Whether this is a type-only re-export.
    pub is_type_only: bool,
}

/// A dynamic `import()` call.
#[derive(Debug, Clone)]
pub struct DynamicImportInfo {
    /// The import specifier.
    pub source: String,
    /// Source span of the `import()` expression.
    pub span: Span,
    /// Names destructured from the dynamic import result.
    /// Non-empty means `const { a, b } = await import(...)` -> Named imports.
    /// Empty means simple `import(...)` or `const x = await import(...)` -> Namespace.
    pub destructured_names: Vec<String>,
    /// The local variable name for `const x = await import(...)`.
    /// Used for namespace import narrowing via member access tracking.
    pub local_name: Option<String>,
}

/// A `require()` call.
#[derive(Debug, Clone)]
pub struct RequireCallInfo {
    /// The require specifier.
    pub source: String,
    /// Source span of the `require()` call.
    pub span: Span,
    /// Names destructured from the `require()` result.
    /// Non-empty means `const { a, b } = require(...)` -> Named imports.
    /// Empty means simple `require(...)` or `const x = require(...)` -> Namespace.
    pub destructured_names: Vec<String>,
    /// The local variable name for `const x = require(...)`.
    /// Used for namespace import narrowing via member access tracking.
    pub local_name: Option<String>,
}

/// Result of parsing all files, including incremental cache statistics.
pub struct ParseResult {
    /// Extracted module information for all successfully parsed files.
    pub modules: Vec<ModuleInfo>,
    /// Number of files whose parse results were loaded from cache (unchanged).
    pub cache_hits: usize,
    /// Number of files that required a full parse (new or changed).
    pub cache_misses: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_line_offsets ──────────────────────────────────────────

    #[test]
    fn line_offsets_empty_string() {
        assert_eq!(compute_line_offsets(""), vec![0]);
    }

    #[test]
    fn line_offsets_single_line_no_newline() {
        assert_eq!(compute_line_offsets("hello"), vec![0]);
    }

    #[test]
    fn line_offsets_single_line_with_newline() {
        // "hello\n" => line 0 starts at 0, line 1 starts at 6
        assert_eq!(compute_line_offsets("hello\n"), vec![0, 6]);
    }

    #[test]
    fn line_offsets_multiple_lines() {
        // "abc\ndef\nghi"
        // line 0: offset 0 ("abc")
        // line 1: offset 4 ("def")
        // line 2: offset 8 ("ghi")
        assert_eq!(compute_line_offsets("abc\ndef\nghi"), vec![0, 4, 8]);
    }

    #[test]
    fn line_offsets_trailing_newline() {
        // "abc\ndef\n"
        // line 0: offset 0, line 1: offset 4, line 2: offset 8 (empty line after trailing \n)
        assert_eq!(compute_line_offsets("abc\ndef\n"), vec![0, 4, 8]);
    }

    #[test]
    fn line_offsets_consecutive_newlines() {
        // "\n\n\n" = 3 newlines => 4 lines
        assert_eq!(compute_line_offsets("\n\n\n"), vec![0, 1, 2, 3]);
    }

    #[test]
    fn line_offsets_multibyte_utf8() {
        // "á\n" => 'á' is 2 bytes (0xC3 0xA1), '\n' at byte 2 => next line at byte 3
        assert_eq!(compute_line_offsets("á\n"), vec![0, 3]);
    }

    // ── byte_offset_to_line_col ──────────────────────────────────────

    #[test]
    fn line_col_offset_zero() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        let (line, col) = byte_offset_to_line_col(&offsets, 0);
        assert_eq!((line, col), (1, 0)); // line 1, col 0
    }

    #[test]
    fn line_col_middle_of_first_line() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        let (line, col) = byte_offset_to_line_col(&offsets, 2);
        assert_eq!((line, col), (1, 2)); // 'c' in "abc"
    }

    #[test]
    fn line_col_start_of_second_line() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        // byte 4 = start of "def"
        let (line, col) = byte_offset_to_line_col(&offsets, 4);
        assert_eq!((line, col), (2, 0));
    }

    #[test]
    fn line_col_middle_of_second_line() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        // byte 5 = 'e' in "def"
        let (line, col) = byte_offset_to_line_col(&offsets, 5);
        assert_eq!((line, col), (2, 1));
    }

    #[test]
    fn line_col_start_of_third_line() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        // byte 8 = start of "ghi"
        let (line, col) = byte_offset_to_line_col(&offsets, 8);
        assert_eq!((line, col), (3, 0));
    }

    #[test]
    fn line_col_end_of_file() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        // byte 10 = 'i' (last char)
        let (line, col) = byte_offset_to_line_col(&offsets, 10);
        assert_eq!((line, col), (3, 2));
    }

    #[test]
    fn line_col_single_line() {
        let offsets = compute_line_offsets("hello");
        let (line, col) = byte_offset_to_line_col(&offsets, 3);
        assert_eq!((line, col), (1, 3));
    }

    #[test]
    fn line_col_at_newline_byte() {
        let offsets = compute_line_offsets("abc\ndef");
        // byte 3 = the '\n' character itself, still part of line 1
        let (line, col) = byte_offset_to_line_col(&offsets, 3);
        assert_eq!((line, col), (1, 3));
    }

    // ── ExportName ───────────────────────────────────────────────────

    #[test]
    fn export_name_matches_str_named() {
        let name = ExportName::Named("foo".to_string());
        assert!(name.matches_str("foo"));
        assert!(!name.matches_str("bar"));
        assert!(!name.matches_str("default"));
    }

    #[test]
    fn export_name_matches_str_default() {
        let name = ExportName::Default;
        assert!(name.matches_str("default"));
        assert!(!name.matches_str("foo"));
    }

    #[test]
    fn export_name_display_named() {
        let name = ExportName::Named("myExport".to_string());
        assert_eq!(name.to_string(), "myExport");
    }

    #[test]
    fn export_name_display_default() {
        let name = ExportName::Default;
        assert_eq!(name.to_string(), "default");
    }
}
