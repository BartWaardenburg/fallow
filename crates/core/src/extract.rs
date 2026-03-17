use std::path::Path;

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
    pub require_calls: Vec<RequireCallInfo>,
    pub member_accesses: Vec<MemberAccess>,
    pub has_cjs_exports: bool,
    pub content_hash: u64,
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
pub enum MemberKind {
    EnumMember,
    ClassMethod,
    ClassProperty,
}

/// A static member access expression (e.g., `Status.Active`, `MyClass.create()`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    parse_source_to_module(file.id, &file.path, &source, content_hash)
}

/// Parse a single file and extract module information.
pub fn parse_single_file(file: &DiscoveredFile) -> Option<ModuleInfo> {
    let source = std::fs::read_to_string(&file.path).ok()?;
    let content_hash = xxhash_rust::xxh3::xxh3_64(source.as_bytes());
    parse_source_to_module(file.id, &file.path, &source, content_hash)
}

/// Parse source text into a ModuleInfo.
fn parse_source_to_module(
    file_id: FileId,
    path: &Path,
    source: &str,
    content_hash: u64,
) -> Option<ModuleInfo> {
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, source, source_type).parse();

    // Extract imports/exports even if there are parse errors
    let mut extractor = ModuleInfoExtractor::new();
    extractor.visit_program(&parser_return.program);

    Some(ModuleInfo {
        file_id,
        exports: extractor.exports,
        imports: extractor.imports,
        re_exports: extractor.re_exports,
        dynamic_imports: extractor.dynamic_imports,
        require_calls: extractor.require_calls,
        member_accesses: extractor.member_accesses,
        has_cjs_exports: extractor.has_cjs_exports,
        content_hash,
    })
}

/// Parse from in-memory content (for LSP).
pub fn parse_from_content(file_id: FileId, path: &Path, content: &str) -> ModuleInfo {
    let content_hash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, content, source_type).parse();

    let mut extractor = ModuleInfoExtractor::new();
    extractor.visit_program(&parser_return.program);

    ModuleInfo {
        file_id,
        exports: extractor.exports,
        imports: extractor.imports,
        re_exports: extractor.re_exports,
        dynamic_imports: extractor.dynamic_imports,
        require_calls: extractor.require_calls,
        member_accesses: extractor.member_accesses,
        has_cjs_exports: extractor.has_cjs_exports,
        content_hash,
    }
}

/// Extract class members (methods and properties) from a class declaration.
fn extract_class_members(class: &Class<'_>) -> Vec<MemberInfo> {
    let mut members = Vec::new();
    for element in &class.body.body {
        match element {
            ClassElement::MethodDefinition(method) => {
                if let Some(name) = method.key.static_name() {
                    let name_str = name.to_string();
                    // Skip constructor and private methods
                    if name_str != "constructor" {
                        members.push(MemberInfo {
                            name: name_str,
                            kind: MemberKind::ClassMethod,
                            span: method.span,
                        });
                    }
                }
            }
            ClassElement::PropertyDefinition(prop) => {
                if let Some(name) = prop.key.static_name() {
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

/// AST visitor that extracts all import/export information in a single pass.
struct ModuleInfoExtractor {
    exports: Vec<ExportInfo>,
    imports: Vec<ImportInfo>,
    re_exports: Vec<ReExportInfo>,
    dynamic_imports: Vec<DynamicImportInfo>,
    require_calls: Vec<RequireCallInfo>,
    member_accesses: Vec<MemberAccess>,
    has_cjs_exports: bool,
}

impl ModuleInfoExtractor {
    fn new() -> Self {
        Self {
            exports: Vec::new(),
            imports: Vec::new(),
            re_exports: Vec::new(),
            dynamic_imports: Vec::new(),
            require_calls: Vec::new(),
            member_accesses: Vec::new(),
            has_cjs_exports: false,
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
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        self.exports.push(ExportInfo {
            name: ExportName::Default,
            local_name: None,
            is_type_only: false,
            span: decl.span,
            members: vec![],
        });
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
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        // Detect dynamic import()
        if let Expression::StringLiteral(lit) = &expr.source {
            self.dynamic_imports.push(DynamicImportInfo {
                source: lit.value.to_string(),
                span: expr.span,
            });
        }

        walk::walk_import_expression(self, expr);
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        // Detect require()
        if let Expression::Identifier(ident) = &expr.callee
            && ident.name == "require"
            && let Some(Argument::StringLiteral(lit)) = expr.arguments.first()
        {
            self.require_calls.push(RequireCallInfo {
                source: lit.value.to_string(),
                span: expr.span,
            });
        }

        walk::walk_call_expression(self, expr);
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        // Detect module.exports = ... and exports.foo = ...
        if let AssignmentTarget::StaticMemberExpression(member) = &expr.left
            && let Expression::Identifier(obj) = &member.object
        {
            if obj.name == "module" && member.property.name == "exports" {
                self.has_cjs_exports = true;
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
        walk::walk_static_member_expression(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_source(source: &str) -> ModuleInfo {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("test.ts").unwrap();
        let parser_return = Parser::new(&allocator, source, source_type).parse();

        let mut extractor = ModuleInfoExtractor::new();
        extractor.visit_program(&parser_return.program);

        ModuleInfo {
            file_id: FileId(0),
            exports: extractor.exports,
            imports: extractor.imports,
            re_exports: extractor.re_exports,
            dynamic_imports: extractor.dynamic_imports,
            require_calls: extractor.require_calls,
            member_accesses: extractor.member_accesses,
            has_cjs_exports: extractor.has_cjs_exports,
            content_hash: 0,
        }
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
}
