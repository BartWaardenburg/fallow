use oxc_span::Span;

use crate::discover::FileId;
use crate::suppress::Suppression;

/// Extracted module information from a single file.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub file_id: FileId,
    pub exports: Vec<ExportInfo>,
    pub imports: Vec<ImportInfo>,
    pub re_exports: Vec<ReExportInfo>,
    pub dynamic_imports: Vec<DynamicImportInfo>,
    pub dynamic_import_patterns: Vec<DynamicImportPattern>,
    pub require_calls: Vec<RequireCallInfo>,
    pub member_accesses: Vec<MemberAccess>,
    /// Identifiers used in "all members consumed" patterns
    /// (Object.values, Object.keys, Object.entries, for..in, spread, computed dynamic access).
    pub whole_object_uses: Vec<String>,
    pub has_cjs_exports: bool,
    pub content_hash: u64,
    /// Inline suppression directives parsed from comments.
    pub suppressions: Vec<Suppression>,
}

/// A dynamic import with a pattern that can be partially resolved (e.g., template literals).
#[derive(Debug, Clone)]
pub struct DynamicImportPattern {
    /// Static prefix of the import path (e.g., "./locales/"). May contain glob characters.
    pub prefix: String,
    /// Static suffix of the import path (e.g., ".json"), if any.
    pub suffix: Option<String>,
    pub span: Span,
}

/// An export declaration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportInfo {
    pub name: ExportName,
    pub local_name: Option<String>,
    pub is_type_only: bool,
    #[serde(serialize_with = "serialize_span")]
    pub span: Span,
    /// Members of this export (for enums and classes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberInfo>,
}

/// A member of an enum or class.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemberInfo {
    pub name: String,
    pub kind: MemberKind,
    #[serde(serialize_with = "serialize_span")]
    pub span: Span,
    /// Whether this member has decorators (e.g., `@Column()`, `@Inject()`).
    /// Decorated members are used by frameworks at runtime and should not be
    /// flagged as unused class members.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_decorator: bool,
}

/// The kind of member.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    EnumMember,
    ClassMethod,
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

#[allow(clippy::trivially_copy_pass_by_ref)] // serde serialize_with requires &T
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
    Named(String),
    Default,
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
    pub source: String,
    pub imported_name: ImportedName,
    pub local_name: String,
    pub is_type_only: bool,
    pub span: Span,
}

/// How a symbol is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportedName {
    Named(String),
    Default,
    Namespace,
    SideEffect,
}

/// A re-export declaration.
#[derive(Debug, Clone)]
pub struct ReExportInfo {
    pub source: String,
    pub imported_name: String,
    pub exported_name: String,
    pub is_type_only: bool,
}

/// A dynamic `import()` call.
#[derive(Debug, Clone)]
pub struct DynamicImportInfo {
    pub source: String,
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
    pub source: String,
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
