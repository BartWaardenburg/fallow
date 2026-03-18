use std::path::Path;
use std::sync::LazyLock;

use fallow_config::ResolvedConfig;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use rayon::prelude::*;

use crate::cache::CacheStore;
use crate::discover::{DiscoveredFile, FileId};

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
}

/// A `require()` call.
#[derive(Debug, Clone)]
pub struct RequireCallInfo {
    pub source: String,
    pub span: Span,
    /// Names destructured from the require() result.
    /// Non-empty means `const { a, b } = require(...)` → Named imports.
    /// Empty means simple `require(...)` or `const x = require(...)` → Namespace.
    pub destructured_names: Vec<String>,
    /// The local variable name for `const x = require(...)`.
    /// Used for namespace import narrowing via member access tracking.
    pub local_name: Option<String>,
}

/// Parse all files in parallel, extracting imports and exports.
/// Uses the cache to skip reparsing files whose content hasn't changed.
pub fn parse_all_files(
    files: &[DiscoveredFile],
    _config: &ResolvedConfig,
    cache: Option<&CacheStore>,
) -> Vec<ModuleInfo> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let cache_hits = AtomicUsize::new(0);
    let cache_misses = AtomicUsize::new(0);

    let result: Vec<ModuleInfo> = files
        .par_iter()
        .filter_map(|file| parse_single_file_cached(file, cache, &cache_hits, &cache_misses))
        .collect();

    let hits = cache_hits.load(Ordering::Relaxed);
    let misses = cache_misses.load(Ordering::Relaxed);
    if hits > 0 || misses > 0 {
        tracing::info!(
            cache_hits = hits,
            cache_misses = misses,
            "incremental cache stats"
        );
    }

    result
}

/// Parse a single file, consulting the cache first.
fn parse_single_file_cached(
    file: &DiscoveredFile,
    cache: Option<&CacheStore>,
    cache_hits: &std::sync::atomic::AtomicUsize,
    cache_misses: &std::sync::atomic::AtomicUsize,
) -> Option<ModuleInfo> {
    use std::sync::atomic::Ordering;

    let source = std::fs::read_to_string(&file.path).ok()?;
    let content_hash = xxhash_rust::xxh3::xxh3_64(source.as_bytes());

    // Check cache before parsing
    if let Some(store) = cache
        && let Some(cached) = store.get(&file.path, content_hash)
    {
        cache_hits.fetch_add(1, Ordering::Relaxed);
        return Some(crate::cache::cached_to_module(cached, file.id));
    }
    cache_misses.fetch_add(1, Ordering::Relaxed);

    // Cache miss — do a full parse
    Some(parse_source_to_module(
        file.id,
        &file.path,
        &source,
        content_hash,
    ))
}

/// Parse a single file and extract module information.
pub fn parse_single_file(file: &DiscoveredFile) -> Option<ModuleInfo> {
    let source = std::fs::read_to_string(&file.path).ok()?;
    let content_hash = xxhash_rust::xxh3::xxh3_64(source.as_bytes());
    Some(parse_source_to_module(
        file.id,
        &file.path,
        &source,
        content_hash,
    ))
}

/// Regex to extract `<script>` block content from Vue/Svelte SFCs.
static SCRIPT_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?is)<script\b(?P<attrs>[^>]*)>(?P<body>[\s\S]*?)</script>"#)
        .expect("valid regex")
});

/// Regex to extract the `lang` attribute value from a script tag.
static LANG_ATTR_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"lang\s*=\s*["'](\w+)["']"#).expect("valid regex"));

pub(crate) struct SfcScript {
    pub body: String,
    pub is_typescript: bool,
    /// Byte offset of the script body within the full SFC source.
    pub byte_offset: usize,
}

pub(crate) fn extract_sfc_scripts(source: &str) -> Vec<SfcScript> {
    SCRIPT_BLOCK_RE
        .captures_iter(source)
        .map(|cap| {
            let attrs = cap.name("attrs").map(|m| m.as_str()).unwrap_or("");
            let body_match = cap.name("body");
            let byte_offset = body_match.map(|m| m.start()).unwrap_or(0);
            let body = body_match.map(|m| m.as_str()).unwrap_or("").to_string();
            let is_typescript = LANG_ATTR_RE
                .captures(attrs)
                .and_then(|c| c.get(1))
                .map(|m| matches!(m.as_str(), "ts" | "tsx"))
                .unwrap_or(false);
            SfcScript {
                body,
                is_typescript,
                byte_offset,
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
fn parse_sfc_to_module(file_id: FileId, source: &str, content_hash: u64) -> ModuleInfo {
    let scripts = extract_sfc_scripts(source);

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
    };

    for script in &scripts {
        let source_type = if script.is_typescript {
            SourceType::ts()
        } else {
            SourceType::mjs()
        };
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &script.body, source_type).parse();
        let mut extractor = ModuleInfoExtractor::new();
        extractor.visit_program(&parser_return.program);

        combined.imports.extend(extractor.imports);
        combined.exports.extend(extractor.exports);
        combined.re_exports.extend(extractor.re_exports);
        combined.dynamic_imports.extend(extractor.dynamic_imports);
        combined
            .dynamic_import_patterns
            .extend(extractor.dynamic_import_patterns);
        combined.require_calls.extend(extractor.require_calls);
        combined.member_accesses.extend(extractor.member_accesses);
        combined
            .whole_object_uses
            .extend(extractor.whole_object_uses);
        combined.has_cjs_exports |= extractor.has_cjs_exports;
    }

    combined
}

/// Parse source text into a ModuleInfo.
fn parse_source_to_module(
    file_id: FileId,
    path: &Path,
    source: &str,
    content_hash: u64,
) -> ModuleInfo {
    if is_sfc_file(path) {
        return parse_sfc_to_module(file_id, source, content_hash);
    }

    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, source, source_type).parse();

    // Extract imports/exports even if there are parse errors
    let mut extractor = ModuleInfoExtractor::new();
    extractor.visit_program(&parser_return.program);

    ModuleInfo {
        file_id,
        exports: extractor.exports,
        imports: extractor.imports,
        re_exports: extractor.re_exports,
        dynamic_imports: extractor.dynamic_imports,
        dynamic_import_patterns: extractor.dynamic_import_patterns,
        require_calls: extractor.require_calls,
        member_accesses: extractor.member_accesses,
        whole_object_uses: extractor.whole_object_uses,
        has_cjs_exports: extractor.has_cjs_exports,
        content_hash,
    }
}

/// Parse from in-memory content (for LSP).
pub fn parse_from_content(file_id: FileId, path: &Path, content: &str) -> ModuleInfo {
    let content_hash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
    parse_source_to_module(file_id, path, content, content_hash)
}

/// Extract class members (methods and properties) from a class declaration.
fn extract_class_members(class: &Class<'_>) -> Vec<MemberInfo> {
    let mut members = Vec::new();
    for element in &class.body.body {
        match element {
            ClassElement::MethodDefinition(method) => {
                if let Some(name) = method.key.static_name() {
                    let name_str = name.to_string();
                    // Skip constructor, private, and protected methods
                    if name_str != "constructor"
                        && !matches!(
                            method.accessibility,
                            Some(oxc_ast::ast::TSAccessibility::Private)
                                | Some(oxc_ast::ast::TSAccessibility::Protected)
                        )
                    {
                        members.push(MemberInfo {
                            name: name_str,
                            kind: MemberKind::ClassMethod,
                            span: method.span,
                        });
                    }
                }
            }
            ClassElement::PropertyDefinition(prop) => {
                if let Some(name) = prop.key.static_name()
                    && !matches!(
                        prop.accessibility,
                        Some(oxc_ast::ast::TSAccessibility::Private)
                            | Some(oxc_ast::ast::TSAccessibility::Protected)
                    )
                {
                    members.push(MemberInfo {
                        name: name.to_string(),
                        kind: MemberKind::ClassProperty,
                        span: prop.span,
                    });
                }
            }
            _ => {}
        }
    }
    members
}

/// Check if an argument expression is `import.meta.url`.
fn is_meta_url_arg(arg: &Argument<'_>) -> bool {
    if let Argument::StaticMemberExpression(member) = arg
        && member.property.name == "url"
        && matches!(member.object, Expression::MetaProperty(_))
    {
        return true;
    }
    false
}

/// AST visitor that extracts all import/export information in a single pass.
struct ModuleInfoExtractor {
    exports: Vec<ExportInfo>,
    imports: Vec<ImportInfo>,
    re_exports: Vec<ReExportInfo>,
    dynamic_imports: Vec<DynamicImportInfo>,
    dynamic_import_patterns: Vec<DynamicImportPattern>,
    require_calls: Vec<RequireCallInfo>,
    member_accesses: Vec<MemberAccess>,
    whole_object_uses: Vec<String>,
    has_cjs_exports: bool,
    /// Spans of require() calls already handled via destructured require detection.
    handled_require_spans: Vec<Span>,
}

impl ModuleInfoExtractor {
    fn new() -> Self {
        Self {
            exports: Vec::new(),
            imports: Vec::new(),
            re_exports: Vec::new(),
            dynamic_imports: Vec::new(),
            dynamic_import_patterns: Vec::new(),
            require_calls: Vec::new(),
            member_accesses: Vec::new(),
            whole_object_uses: Vec::new(),
            has_cjs_exports: false,
            handled_require_spans: Vec::new(),
        }
    }

    fn extract_declaration_exports(&mut self, decl: &Declaration<'_>, is_type_only: bool) {
        match decl {
            Declaration::VariableDeclaration(var) => {
                for declarator in &var.declarations {
                    self.extract_binding_pattern_names(&declarator.id, is_type_only);
                }
            }
            Declaration::FunctionDeclaration(func) => {
                if let Some(id) = func.id.as_ref() {
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(id.name.to_string()),
                        local_name: Some(id.name.to_string()),
                        is_type_only,
                        span: id.span,
                        members: vec![],
                    });
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = class.id.as_ref() {
                    let members = extract_class_members(class);
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(id.name.to_string()),
                        local_name: Some(id.name.to_string()),
                        is_type_only,
                        span: id.span,
                        members,
                    });
                }
            }
            Declaration::TSTypeAliasDeclaration(alias) => {
                self.exports.push(ExportInfo {
                    name: ExportName::Named(alias.id.name.to_string()),
                    local_name: Some(alias.id.name.to_string()),
                    is_type_only: true,
                    span: alias.id.span,
                    members: vec![],
                });
            }
            Declaration::TSInterfaceDeclaration(iface) => {
                self.exports.push(ExportInfo {
                    name: ExportName::Named(iface.id.name.to_string()),
                    local_name: Some(iface.id.name.to_string()),
                    is_type_only: true,
                    span: iface.id.span,
                    members: vec![],
                });
            }
            Declaration::TSEnumDeclaration(enumd) => {
                let members: Vec<MemberInfo> = enumd
                    .body
                    .members
                    .iter()
                    .filter_map(|member| {
                        let name = match &member.id {
                            TSEnumMemberName::Identifier(id) => id.name.to_string(),
                            TSEnumMemberName::String(s) | TSEnumMemberName::ComputedString(s) => {
                                s.value.to_string()
                            }
                            TSEnumMemberName::ComputedTemplateString(_) => return None,
                        };
                        Some(MemberInfo {
                            name,
                            kind: MemberKind::EnumMember,
                            span: member.span,
                        })
                    })
                    .collect();
                self.exports.push(ExportInfo {
                    name: ExportName::Named(enumd.id.name.to_string()),
                    local_name: Some(enumd.id.name.to_string()),
                    is_type_only,
                    span: enumd.id.span,
                    members,
                });
            }
            Declaration::TSModuleDeclaration(module) => match &module.id {
                TSModuleDeclarationName::Identifier(id) => {
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(id.name.to_string()),
                        local_name: Some(id.name.to_string()),
                        is_type_only: true,
                        span: id.span,
                        members: vec![],
                    });
                }
                TSModuleDeclarationName::StringLiteral(lit) => {
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(lit.value.to_string()),
                        local_name: Some(lit.value.to_string()),
                        is_type_only: true,
                        span: lit.span,
                        members: vec![],
                    });
                }
            },
            _ => {}
        }
    }

    fn extract_binding_pattern_names(&mut self, pattern: &BindingPattern<'_>, is_type_only: bool) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                self.exports.push(ExportInfo {
                    name: ExportName::Named(id.name.to_string()),
                    local_name: Some(id.name.to_string()),
                    is_type_only,
                    span: id.span,
                    members: vec![],
                });
            }
            BindingPattern::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    self.extract_binding_pattern_names(&prop.value, is_type_only);
                }
            }
            BindingPattern::ArrayPattern(arr) => {
                for elem in arr.elements.iter().flatten() {
                    self.extract_binding_pattern_names(elem, is_type_only);
                }
            }
            BindingPattern::AssignmentPattern(assign) => {
                self.extract_binding_pattern_names(&assign.left, is_type_only);
            }
        }
    }
}

impl<'a> Visit<'a> for ModuleInfoExtractor {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.to_string();
        let is_type_only = decl.import_kind.is_type();

        if let Some(specifiers) = &decl.specifiers {
            for spec in specifiers {
                match spec {
                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        self.imports.push(ImportInfo {
                            source: source.clone(),
                            imported_name: ImportedName::Named(s.imported.name().to_string()),
                            local_name: s.local.name.to_string(),
                            is_type_only: is_type_only || s.import_kind.is_type(),
                            span: s.span,
                        });
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        self.imports.push(ImportInfo {
                            source: source.clone(),
                            imported_name: ImportedName::Default,
                            local_name: s.local.name.to_string(),
                            is_type_only,
                            span: s.span,
                        });
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        self.imports.push(ImportInfo {
                            source: source.clone(),
                            imported_name: ImportedName::Namespace,
                            local_name: s.local.name.to_string(),
                            is_type_only,
                            span: s.span,
                        });
                    }
                }
            }
        } else {
            // Side-effect import: import './styles.css'
            self.imports.push(ImportInfo {
                source,
                imported_name: ImportedName::SideEffect,
                local_name: String::new(),
                is_type_only: false,
                span: decl.span,
            });
        }
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        let is_type_only = decl.export_kind.is_type();

        if let Some(source) = &decl.source {
            // Re-export: export { foo } from './bar'
            for spec in &decl.specifiers {
                self.re_exports.push(ReExportInfo {
                    source: source.value.to_string(),
                    imported_name: spec.local.name().to_string(),
                    exported_name: spec.exported.name().to_string(),
                    is_type_only: is_type_only || spec.export_kind.is_type(),
                });
            }
        } else {
            // Local export
            if let Some(declaration) = &decl.declaration {
                self.extract_declaration_exports(declaration, is_type_only);
            }
            for spec in &decl.specifiers {
                self.exports.push(ExportInfo {
                    name: ExportName::Named(spec.exported.name().to_string()),
                    local_name: Some(spec.local.name().to_string()),
                    is_type_only: is_type_only || spec.export_kind.is_type(),
                    span: spec.span,
                    members: vec![],
                });
            }
        }

        walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        self.exports.push(ExportInfo {
            name: ExportName::Default,
            local_name: None,
            is_type_only: false,
            span: decl.span,
            members: vec![],
        });

        walk::walk_export_default_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let exported_name = decl
            .exported
            .as_ref()
            .map(|e| e.name().to_string())
            .unwrap_or_else(|| "*".to_string());

        self.re_exports.push(ReExportInfo {
            source: decl.source.value.to_string(),
            imported_name: "*".to_string(),
            exported_name,
            is_type_only: decl.export_kind.is_type(),
        });

        walk::walk_export_all_declaration(self, decl);
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        match &expr.source {
            Expression::StringLiteral(lit) => {
                self.dynamic_imports.push(DynamicImportInfo {
                    source: lit.value.to_string(),
                    span: expr.span,
                });
            }
            Expression::TemplateLiteral(tpl)
                if !tpl.quasis.is_empty() && !tpl.expressions.is_empty() =>
            {
                // Template literal with expressions: extract prefix/suffix.
                // For multi-expression templates like `./a/${x}/${y}.js` (3 quasis),
                // use `**/` in the prefix so the glob can match nested directories.
                let first_quasi = tpl.quasis[0].value.raw.to_string();
                if first_quasi.starts_with("./") || first_quasi.starts_with("../") {
                    let prefix = if tpl.expressions.len() > 1 {
                        // Multiple dynamic segments: use ** to match any nesting depth
                        format!("{first_quasi}**/")
                    } else {
                        first_quasi
                    };
                    let suffix = if tpl.quasis.len() > 1 {
                        let last = &tpl.quasis[tpl.quasis.len() - 1];
                        let s = last.value.raw.to_string();
                        if s.is_empty() { None } else { Some(s) }
                    } else {
                        None
                    };
                    self.dynamic_import_patterns.push(DynamicImportPattern {
                        prefix,
                        suffix,
                        span: expr.span,
                    });
                }
            }
            Expression::TemplateLiteral(tpl)
                if !tpl.quasis.is_empty() && tpl.expressions.is_empty() =>
            {
                // No-substitution template literal: treat as exact string
                let value = tpl.quasis[0].value.raw.to_string();
                if !value.is_empty() {
                    self.dynamic_imports.push(DynamicImportInfo {
                        source: value,
                        span: expr.span,
                    });
                }
            }
            Expression::BinaryExpression(bin)
                if bin.operator == oxc_ast::ast::BinaryOperator::Addition =>
            {
                if let Some((prefix, suffix)) = extract_concat_parts(bin)
                    && (prefix.starts_with("./") || prefix.starts_with("../"))
                {
                    self.dynamic_import_patterns.push(DynamicImportPattern {
                        prefix,
                        suffix,
                        span: expr.span,
                    });
                }
            }
            _ => {}
        }

        walk::walk_import_expression(self, expr);
    }

    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        for declarator in &decl.declarations {
            let Some(Expression::CallExpression(call)) = &declarator.init else {
                continue;
            };
            let Expression::Identifier(callee) = &call.callee else {
                continue;
            };
            if callee.name != "require" {
                continue;
            }
            let Some(Argument::StringLiteral(lit)) = call.arguments.first() else {
                continue;
            };
            let source = lit.value.to_string();

            match &declarator.id {
                BindingPattern::ObjectPattern(obj_pat) => {
                    if obj_pat.rest.is_some() {
                        self.require_calls.push(RequireCallInfo {
                            source,
                            span: call.span,
                            destructured_names: Vec::new(),
                            local_name: None,
                        });
                    } else {
                        let names: Vec<String> = obj_pat
                            .properties
                            .iter()
                            .filter_map(|prop| prop.key.static_name().map(|n| n.to_string()))
                            .collect();
                        self.require_calls.push(RequireCallInfo {
                            source,
                            span: call.span,
                            destructured_names: names,
                            local_name: None,
                        });
                    }
                    self.handled_require_spans.push(call.span);
                }
                BindingPattern::BindingIdentifier(id) => {
                    // `const mod = require('./x')` → Namespace with local_name for narrowing
                    self.require_calls.push(RequireCallInfo {
                        source,
                        span: call.span,
                        destructured_names: Vec::new(),
                        local_name: Some(id.name.to_string()),
                    });
                    self.handled_require_spans.push(call.span);
                }
                _ => {}
            }
        }
        walk::walk_variable_declaration(self, decl);
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        // Detect require()
        if let Expression::Identifier(ident) = &expr.callee
            && ident.name == "require"
            && let Some(Argument::StringLiteral(lit)) = expr.arguments.first()
            && !self.handled_require_spans.contains(&expr.span)
        {
            self.require_calls.push(RequireCallInfo {
                source: lit.value.to_string(),
                span: expr.span,
                destructured_names: Vec::new(),
                local_name: None,
            });
        }

        // Detect Object.values(X), Object.keys(X), Object.entries(X) — whole-object use
        if let Expression::StaticMemberExpression(member) = &expr.callee
            && let Expression::Identifier(obj) = &member.object
            && obj.name == "Object"
            && matches!(member.property.name.as_str(), "values" | "keys" | "entries")
            && let Some(Argument::Identifier(arg_ident)) = expr.arguments.first()
        {
            self.whole_object_uses.push(arg_ident.name.to_string());
        }

        // Detect import.meta.glob() — Vite pattern
        if let Expression::StaticMemberExpression(member) = &expr.callee
            && member.property.name == "glob"
            && matches!(member.object, Expression::MetaProperty(_))
            && let Some(first_arg) = expr.arguments.first()
        {
            match first_arg {
                Argument::StringLiteral(lit) => {
                    let s = lit.value.to_string();
                    if s.starts_with("./") || s.starts_with("../") {
                        self.dynamic_import_patterns.push(DynamicImportPattern {
                            prefix: s,
                            suffix: None,
                            span: expr.span,
                        });
                    }
                }
                Argument::ArrayExpression(arr) => {
                    for elem in &arr.elements {
                        if let ArrayExpressionElement::StringLiteral(lit) = elem {
                            let s = lit.value.to_string();
                            if s.starts_with("./") || s.starts_with("../") {
                                self.dynamic_import_patterns.push(DynamicImportPattern {
                                    prefix: s,
                                    suffix: None,
                                    span: expr.span,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Detect require.context() — Webpack pattern
        if let Expression::StaticMemberExpression(member) = &expr.callee
            && member.property.name == "context"
            && let Expression::Identifier(obj) = &member.object
            && obj.name == "require"
            && let Some(Argument::StringLiteral(dir_lit)) = expr.arguments.first()
        {
            let dir = dir_lit.value.to_string();
            if dir.starts_with("./") || dir.starts_with("../") {
                let recursive = expr
                    .arguments
                    .get(1)
                    .is_some_and(|arg| matches!(arg, Argument::BooleanLiteral(b) if b.value));
                let prefix = if recursive {
                    format!("{dir}/**/")
                } else {
                    format!("{dir}/")
                };
                self.dynamic_import_patterns.push(DynamicImportPattern {
                    prefix,
                    suffix: None,
                    span: expr.span,
                });
            }
        }

        walk::walk_call_expression(self, expr);
    }

    fn visit_new_expression(&mut self, expr: &oxc_ast::ast::NewExpression<'a>) {
        // Detect `new URL('./path', import.meta.url)` pattern.
        // This is the standard Vite/bundler pattern for referencing worker files and assets.
        // Treat the path as a dynamic import so the target file is considered reachable.
        if let Expression::Identifier(callee) = &expr.callee
            && callee.name == "URL"
            && expr.arguments.len() == 2
            && let Some(Argument::StringLiteral(path_lit)) = expr.arguments.first()
            && is_meta_url_arg(&expr.arguments[1])
            && (path_lit.value.starts_with("./") || path_lit.value.starts_with("../"))
        {
            self.dynamic_imports.push(DynamicImportInfo {
                source: path_lit.value.to_string(),
                span: expr.span,
            });
        }

        walk::walk_new_expression(self, expr);
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        // Detect module.exports = ... and exports.foo = ...
        if let AssignmentTarget::StaticMemberExpression(member) = &expr.left {
            if let Expression::Identifier(obj) = &member.object {
                if obj.name == "module" && member.property.name == "exports" {
                    self.has_cjs_exports = true;
                    // Extract exports from `module.exports = { foo, bar }`
                    if let Expression::ObjectExpression(obj_expr) = &expr.right {
                        for prop in &obj_expr.properties {
                            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop
                                && let Some(name) = p.key.static_name()
                            {
                                self.exports.push(ExportInfo {
                                    name: ExportName::Named(name.to_string()),
                                    local_name: None,
                                    is_type_only: false,
                                    span: p.span,
                                    members: vec![],
                                });
                            }
                        }
                    }
                }
                if obj.name == "exports" {
                    self.has_cjs_exports = true;
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(member.property.name.to_string()),
                        local_name: None,
                        is_type_only: false,
                        span: expr.span,
                        members: vec![],
                    });
                }
            }
            // Capture `this.member = ...` assignment patterns within class bodies.
            // This indicates the class uses the member internally.
            if matches!(member.object, Expression::ThisExpression(_)) {
                self.member_accesses.push(MemberAccess {
                    object: "this".to_string(),
                    member: member.property.name.to_string(),
                });
            }
        }
        walk::walk_assignment_expression(self, expr);
    }

    fn visit_static_member_expression(&mut self, expr: &StaticMemberExpression<'a>) {
        // Capture `Identifier.member` patterns (e.g., `Status.Active`, `MyClass.create()`)
        if let Expression::Identifier(obj) = &expr.object {
            self.member_accesses.push(MemberAccess {
                object: obj.name.to_string(),
                member: expr.property.name.to_string(),
            });
        }
        // Capture `this.member` patterns within class bodies — these members are used internally
        if matches!(expr.object, Expression::ThisExpression(_)) {
            self.member_accesses.push(MemberAccess {
                object: "this".to_string(),
                member: expr.property.name.to_string(),
            });
        }
        walk::walk_static_member_expression(self, expr);
    }

    fn visit_computed_member_expression(&mut self, expr: &ComputedMemberExpression<'a>) {
        if let Expression::Identifier(obj) = &expr.object {
            if let Expression::StringLiteral(lit) = &expr.expression {
                // Computed access with string literal resolves to a specific member
                self.member_accesses.push(MemberAccess {
                    object: obj.name.to_string(),
                    member: lit.value.to_string(),
                });
            } else {
                // Dynamic computed access — mark all members as used
                self.whole_object_uses.push(obj.name.to_string());
            }
        }
        walk::walk_computed_member_expression(self, expr);
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        if let Expression::Identifier(ident) = &stmt.right {
            self.whole_object_uses.push(ident.name.to_string());
        }
        walk::walk_for_in_statement(self, stmt);
    }

    fn visit_spread_element(&mut self, elem: &SpreadElement<'a>) {
        if let Expression::Identifier(ident) = &elem.argument {
            self.whole_object_uses.push(ident.name.to_string());
        }
        walk::walk_spread_element(self, elem);
    }
}

/// Extract static prefix and optional suffix from a binary addition chain.
fn extract_concat_parts(expr: &BinaryExpression<'_>) -> Option<(String, Option<String>)> {
    let prefix = extract_leading_string(&expr.left)?;
    let suffix = extract_trailing_string(&expr.right);
    Some((prefix, suffix))
}

fn extract_leading_string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(lit) => Some(lit.value.to_string()),
        Expression::BinaryExpression(bin)
            if bin.operator == oxc_ast::ast::BinaryOperator::Addition =>
        {
            extract_leading_string(&bin.left)
        }
        _ => None,
    }
}

fn extract_trailing_string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(lit) => {
            let s = lit.value.to_string();
            if s.is_empty() { None } else { Some(s) }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_source(source: &str) -> ModuleInfo {
        parse_source_to_module(FileId(0), Path::new("test.ts"), source, 0)
    }

    #[test]
    fn extracts_named_exports() {
        let info = parse_source("export const foo = 1; export function bar() {}");
        assert_eq!(info.exports.len(), 2);
        assert_eq!(info.exports[0].name, ExportName::Named("foo".to_string()));
        assert_eq!(info.exports[1].name, ExportName::Named("bar".to_string()));
    }

    #[test]
    fn extracts_default_export() {
        let info = parse_source("export default function main() {}");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, ExportName::Default);
    }

    #[test]
    fn extracts_named_imports() {
        let info = parse_source("import { foo, bar } from './utils';");
        assert_eq!(info.imports.len(), 2);
        assert_eq!(
            info.imports[0].imported_name,
            ImportedName::Named("foo".to_string())
        );
        assert_eq!(info.imports[0].source, "./utils");
    }

    #[test]
    fn extracts_namespace_import() {
        let info = parse_source("import * as utils from './utils';");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].imported_name, ImportedName::Namespace);
    }

    #[test]
    fn extracts_side_effect_import() {
        let info = parse_source("import './styles.css';");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].imported_name, ImportedName::SideEffect);
    }

    #[test]
    fn extracts_re_exports() {
        let info = parse_source("export { foo, bar as baz } from './module';");
        assert_eq!(info.re_exports.len(), 2);
        assert_eq!(info.re_exports[0].imported_name, "foo");
        assert_eq!(info.re_exports[0].exported_name, "foo");
        assert_eq!(info.re_exports[1].imported_name, "bar");
        assert_eq!(info.re_exports[1].exported_name, "baz");
    }

    #[test]
    fn extracts_star_re_export() {
        let info = parse_source("export * from './module';");
        assert_eq!(info.re_exports.len(), 1);
        assert_eq!(info.re_exports[0].imported_name, "*");
        assert_eq!(info.re_exports[0].exported_name, "*");
    }

    #[test]
    fn extracts_dynamic_import() {
        let info = parse_source("const mod = import('./lazy');");
        assert_eq!(info.dynamic_imports.len(), 1);
        assert_eq!(info.dynamic_imports[0].source, "./lazy");
    }

    #[test]
    fn extracts_require_call() {
        let info = parse_source("const fs = require('fs');");
        assert_eq!(info.require_calls.len(), 1);
        assert_eq!(info.require_calls[0].source, "fs");
    }

    #[test]
    fn extracts_type_exports() {
        let info = parse_source("export type Foo = string; export interface Bar { x: number; }");
        assert_eq!(info.exports.len(), 2);
        assert!(info.exports[0].is_type_only);
        assert!(info.exports[1].is_type_only);
    }

    #[test]
    fn extracts_type_only_imports() {
        let info = parse_source("import type { Foo } from './types';");
        assert_eq!(info.imports.len(), 1);
        assert!(info.imports[0].is_type_only);
    }

    #[test]
    fn detects_cjs_module_exports() {
        let info = parse_source("module.exports = { foo: 1 };");
        assert!(info.has_cjs_exports);
    }

    #[test]
    fn detects_cjs_exports_property() {
        let info = parse_source("exports.foo = 42;");
        assert!(info.has_cjs_exports);
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, ExportName::Named("foo".to_string()));
    }

    #[test]
    fn extracts_static_member_accesses() {
        let info = parse_source(
            "import { Status, MyClass } from './types';\nconsole.log(Status.Active);\nMyClass.create();",
        );
        // Should capture: console.log, Status.Active, MyClass.create
        assert!(info.member_accesses.len() >= 2);
        let has_status_active = info
            .member_accesses
            .iter()
            .any(|a| a.object == "Status" && a.member == "Active");
        let has_myclass_create = info
            .member_accesses
            .iter()
            .any(|a| a.object == "MyClass" && a.member == "create");
        assert!(has_status_active, "Should capture Status.Active");
        assert!(has_myclass_create, "Should capture MyClass.create");
    }

    #[test]
    fn extracts_default_import() {
        let info = parse_source("import React from 'react';");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].imported_name, ImportedName::Default);
        assert_eq!(info.imports[0].local_name, "React");
        assert_eq!(info.imports[0].source, "react");
    }

    #[test]
    fn extracts_mixed_import_default_and_named() {
        let info = parse_source("import React, { useState, useEffect } from 'react';");
        assert_eq!(info.imports.len(), 3);
        // Oxc orders: named specifiers first, then default
        assert_eq!(info.imports[0].imported_name, ImportedName::Default);
        assert_eq!(info.imports[0].local_name, "React");
        assert_eq!(
            info.imports[1].imported_name,
            ImportedName::Named("useState".to_string())
        );
        assert_eq!(
            info.imports[2].imported_name,
            ImportedName::Named("useEffect".to_string())
        );
    }

    #[test]
    fn extracts_import_with_alias() {
        let info = parse_source("import { foo as bar } from './utils';");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(
            info.imports[0].imported_name,
            ImportedName::Named("foo".to_string())
        );
        assert_eq!(info.imports[0].local_name, "bar");
    }

    #[test]
    fn extracts_export_specifier_list() {
        let info = parse_source("const foo = 1; const bar = 2; export { foo, bar };");
        assert_eq!(info.exports.len(), 2);
        assert_eq!(info.exports[0].name, ExportName::Named("foo".to_string()));
        assert_eq!(info.exports[1].name, ExportName::Named("bar".to_string()));
    }

    #[test]
    fn extracts_export_with_alias() {
        let info = parse_source("const foo = 1; export { foo as myFoo };");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, ExportName::Named("myFoo".to_string()));
    }

    #[test]
    fn extracts_star_re_export_with_alias() {
        let info = parse_source("export * as utils from './utils';");
        assert_eq!(info.re_exports.len(), 1);
        assert_eq!(info.re_exports[0].imported_name, "*");
        assert_eq!(info.re_exports[0].exported_name, "utils");
    }

    #[test]
    fn extracts_export_class_declaration() {
        let info = parse_source("export class MyService { name: string = ''; }");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(
            info.exports[0].name,
            ExportName::Named("MyService".to_string())
        );
    }

    #[test]
    fn class_constructor_is_excluded() {
        let info = parse_source("export class Foo { constructor() {} greet() {} }");
        assert_eq!(info.exports.len(), 1);
        // Members should NOT include constructor
        let members: Vec<&str> = info.exports[0]
            .members
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert!(
            !members.contains(&"constructor"),
            "constructor should be excluded from members"
        );
        assert!(members.contains(&"greet"), "greet should be included");
    }

    #[test]
    fn extracts_ts_enum_declaration() {
        let info = parse_source("export enum Direction { Up, Down, Left, Right }");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(
            info.exports[0].name,
            ExportName::Named("Direction".to_string())
        );
        assert_eq!(info.exports[0].members.len(), 4);
        assert_eq!(info.exports[0].members[0].kind, MemberKind::EnumMember);
    }

    #[test]
    fn extracts_ts_module_declaration() {
        let info = parse_source("export declare module 'my-module' {}");
        assert_eq!(info.exports.len(), 1);
        assert!(info.exports[0].is_type_only);
    }

    #[test]
    fn extracts_type_only_named_import() {
        let info = parse_source("import { type Foo, Bar } from './types';");
        assert_eq!(info.imports.len(), 2);
        assert!(info.imports[0].is_type_only);
        assert!(!info.imports[1].is_type_only);
    }

    #[test]
    fn extracts_type_re_export() {
        let info = parse_source("export type { Foo } from './types';");
        assert_eq!(info.re_exports.len(), 1);
        assert!(info.re_exports[0].is_type_only);
    }

    #[test]
    fn extracts_destructured_array_export() {
        let info = parse_source("export const [first, second] = [1, 2];");
        assert_eq!(info.exports.len(), 2);
        assert_eq!(info.exports[0].name, ExportName::Named("first".to_string()));
        assert_eq!(
            info.exports[1].name,
            ExportName::Named("second".to_string())
        );
    }

    #[test]
    fn extracts_nested_destructured_export() {
        let info = parse_source("export const { a, b: { c } } = obj;");
        assert_eq!(info.exports.len(), 2);
        assert_eq!(info.exports[0].name, ExportName::Named("a".to_string()));
        assert_eq!(info.exports[1].name, ExportName::Named("c".to_string()));
    }

    #[test]
    fn extracts_default_export_function_expression() {
        let info = parse_source("export default function() { return 42; }");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, ExportName::Default);
    }

    #[test]
    fn export_name_display() {
        assert_eq!(ExportName::Named("foo".to_string()).to_string(), "foo");
        assert_eq!(ExportName::Default.to_string(), "default");
    }

    #[test]
    fn no_exports_no_imports() {
        let info = parse_source("const x = 1; console.log(x);");
        assert!(info.exports.is_empty());
        assert!(info.imports.is_empty());
        assert!(info.re_exports.is_empty());
        assert!(!info.has_cjs_exports);
    }

    #[test]
    fn dynamic_import_non_string_ignored() {
        let info = parse_source("const mod = import(variable);");
        // Dynamic import with non-string literal should not be captured
        assert_eq!(info.dynamic_imports.len(), 0);
    }

    #[test]
    fn multiple_require_calls() {
        let info =
            parse_source("const a = require('a'); const b = require('b'); const c = require('c');");
        assert_eq!(info.require_calls.len(), 3);
    }

    #[test]
    fn extracts_ts_interface() {
        let info = parse_source("export interface Props { name: string; age: number; }");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, ExportName::Named("Props".to_string()));
        assert!(info.exports[0].is_type_only);
    }

    #[test]
    fn extracts_ts_type_alias() {
        let info = parse_source("export type ID = string | number;");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, ExportName::Named("ID".to_string()));
        assert!(info.exports[0].is_type_only);
    }

    #[test]
    fn extracts_member_accesses_inside_exported_functions() {
        let info = parse_source(
            "import { Color } from './types';\nexport const isRed = (c: Color) => c === Color.Red;",
        );
        let has_color_red = info
            .member_accesses
            .iter()
            .any(|a| a.object == "Color" && a.member == "Red");
        assert!(
            has_color_red,
            "Should capture Color.Red inside exported function body"
        );
    }

    // ── Whole-object use detection ──────────────────────────────

    #[test]
    fn detects_object_values_whole_use() {
        let info = parse_source("import { Status } from './types';\nObject.values(Status);");
        assert!(info.whole_object_uses.contains(&"Status".to_string()));
    }

    #[test]
    fn detects_object_keys_whole_use() {
        let info = parse_source("import { Dir } from './types';\nObject.keys(Dir);");
        assert!(info.whole_object_uses.contains(&"Dir".to_string()));
    }

    #[test]
    fn detects_object_entries_whole_use() {
        let info = parse_source("import { E } from './types';\nObject.entries(E);");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    #[test]
    fn detects_for_in_whole_use() {
        let info = parse_source("import { Color } from './types';\nfor (const k in Color) {}");
        assert!(info.whole_object_uses.contains(&"Color".to_string()));
    }

    #[test]
    fn detects_spread_whole_use() {
        let info = parse_source("import { X } from './types';\nconst y = { ...X };");
        assert!(info.whole_object_uses.contains(&"X".to_string()));
    }

    #[test]
    fn computed_member_string_literal_resolves() {
        let info = parse_source("import { Status } from './types';\nStatus[\"Active\"];");
        let has_access = info
            .member_accesses
            .iter()
            .any(|a| a.object == "Status" && a.member == "Active");
        assert!(
            has_access,
            "Status[\"Active\"] should resolve to a static member access"
        );
    }

    #[test]
    fn computed_member_variable_marks_whole_use() {
        let info = parse_source("import { Status } from './types';\nconst k = 'foo';\nStatus[k];");
        assert!(info.whole_object_uses.contains(&"Status".to_string()));
    }

    // ── Dynamic import pattern extraction ───────────────────────

    #[test]
    fn extracts_template_literal_dynamic_import_pattern() {
        let info = parse_source("const m = import(`./locales/${lang}.json`);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./locales/");
        assert_eq!(
            info.dynamic_import_patterns[0].suffix,
            Some(".json".to_string())
        );
    }

    #[test]
    fn extracts_concat_dynamic_import_pattern() {
        let info = parse_source("const m = import('./pages/' + name);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./pages/");
        assert!(info.dynamic_import_patterns[0].suffix.is_none());
    }

    #[test]
    fn extracts_concat_with_suffix() {
        let info = parse_source("const m = import('./pages/' + name + '.tsx');");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./pages/");
        assert_eq!(
            info.dynamic_import_patterns[0].suffix,
            Some(".tsx".to_string())
        );
    }

    #[test]
    fn no_substitution_template_treated_as_exact() {
        let info = parse_source("const m = import(`./exact-module`);");
        assert_eq!(info.dynamic_imports.len(), 1);
        assert_eq!(info.dynamic_imports[0].source, "./exact-module");
        assert!(info.dynamic_import_patterns.is_empty());
    }

    #[test]
    fn fully_dynamic_import_still_ignored() {
        let info = parse_source("const m = import(variable);");
        assert!(info.dynamic_imports.is_empty());
        assert!(info.dynamic_import_patterns.is_empty());
    }

    #[test]
    fn non_relative_template_ignored() {
        let info = parse_source("const m = import(`lodash/${fn}`);");
        assert!(info.dynamic_import_patterns.is_empty());
    }

    #[test]
    fn multi_expression_template_uses_globstar() {
        // `./plugins/${cat}/${name}.js` has 2 expressions → prefix gets **/
        let info = parse_source("const m = import(`./plugins/${cat}/${name}.js`);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./plugins/**/");
        assert_eq!(
            info.dynamic_import_patterns[0].suffix,
            Some(".js".to_string())
        );
    }

    // ── Vue/Svelte SFC parsing ──────────────────────────────────

    fn parse_sfc(source: &str, filename: &str) -> ModuleInfo {
        parse_source_to_module(FileId(0), Path::new(filename), source, 0)
    }

    #[test]
    fn extracts_vue_script_imports() {
        let info = parse_sfc(
            r#"
<script lang="ts">
import { ref } from 'vue';
import { helper } from './utils';
export default {};
</script>
<template><div></div></template>
"#,
            "App.vue",
        );
        assert_eq!(info.imports.len(), 2);
        assert!(info.imports.iter().any(|i| i.source == "vue"));
        assert!(info.imports.iter().any(|i| i.source == "./utils"));
    }

    #[test]
    fn extracts_vue_script_setup_imports() {
        let info = parse_sfc(
            r#"
<script setup lang="ts">
import { ref } from 'vue';
const count = ref(0);
</script>
"#,
            "Comp.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn extracts_vue_both_scripts() {
        let info = parse_sfc(
            r#"
<script lang="ts">
import { defineComponent } from 'vue';
export default defineComponent({});
</script>
<script setup lang="ts">
import { ref } from 'vue';
const count = ref(0);
</script>
"#,
            "Dual.vue",
        );
        assert!(info.imports.len() >= 2);
    }

    #[test]
    fn extracts_svelte_script_imports() {
        let info = parse_sfc(
            r#"
<script lang="ts">
import { onMount } from 'svelte';
import { helper } from './utils';
</script>
<p>Hello</p>
"#,
            "App.svelte",
        );
        assert_eq!(info.imports.len(), 2);
        assert!(info.imports.iter().any(|i| i.source == "svelte"));
        assert!(info.imports.iter().any(|i| i.source == "./utils"));
    }

    #[test]
    fn vue_no_script_returns_empty() {
        let info = parse_sfc(
            "<template><div></div></template><style>div {}</style>",
            "NoScript.vue",
        );
        assert!(info.imports.is_empty());
        assert!(info.exports.is_empty());
    }

    #[test]
    fn vue_js_default_lang() {
        let info = parse_sfc(
            r#"
<script>
import { createApp } from 'vue';
export default {};
</script>
"#,
            "JsVue.vue",
        );
        assert_eq!(info.imports.len(), 1);
    }

    // ── import.meta.glob / require.context ──────────────────────

    #[test]
    fn extracts_import_meta_glob_pattern() {
        let info = parse_source("const mods = import.meta.glob('./components/*.tsx');");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./components/*.tsx");
    }

    #[test]
    fn extracts_import_meta_glob_array() {
        let info =
            parse_source("const mods = import.meta.glob(['./pages/*.ts', './layouts/*.ts']);");
        assert_eq!(info.dynamic_import_patterns.len(), 2);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./pages/*.ts");
        assert_eq!(info.dynamic_import_patterns[1].prefix, "./layouts/*.ts");
    }

    #[test]
    fn extracts_require_context_pattern() {
        let info = parse_source("const ctx = require.context('./icons', false);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./icons/");
    }

    #[test]
    fn extracts_require_context_recursive() {
        let info = parse_source("const ctx = require.context('./icons', true);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./icons/**/");
    }
}
