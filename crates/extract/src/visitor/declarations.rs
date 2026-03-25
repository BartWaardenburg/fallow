//! Declaration extraction helpers for `ModuleInfoExtractor`.
//!
//! These are inherent methods that extract export information from
//! declaration AST nodes, binding patterns, and require/import patterns.

use oxc_ast::ast::*;

use crate::{
    DynamicImportInfo, DynamicImportPattern, ExportInfo, ExportName, MemberKind, MemberInfo,
    RequireCallInfo,
};

use super::helpers::extract_class_members;
use super::{extract_destructured_names, ModuleInfoExtractor, MemberAccess};

impl ModuleInfoExtractor {
    pub(crate) fn extract_declaration_exports(
        &mut self,
        decl: &Declaration<'_>,
        is_type_only: bool,
    ) {
        match decl {
            Declaration::VariableDeclaration(var) => {
                for declarator in &var.declarations {
                    self.extract_binding_pattern_names(&declarator.id, is_type_only);
                }
            }
            Declaration::FunctionDeclaration(func) => {
                if let Some(id) = func.id.as_ref() {
                    let name = ExportName::Named(id.name.to_string());
                    // Check if this is an overload — same-named export already exists
                    if let Some(existing) = self.exports.iter_mut().find(|e| e.name == name) {
                        // Update to the implementation (last declaration wins)
                        existing.span = id.span;
                        existing.is_type_only = is_type_only;
                    } else {
                        self.exports.push(ExportInfo {
                            name,
                            local_name: Some(id.name.to_string()),
                            is_type_only,
                            is_public: false,
                            span: id.span,
                            members: vec![],
                        });
                    }
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = class.id.as_ref() {
                    let members = extract_class_members(class);
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(id.name.to_string()),
                        local_name: Some(id.name.to_string()),
                        is_type_only,
                        is_public: false,
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
                    is_public: false,
                    span: alias.id.span,
                    members: vec![],
                });
            }
            Declaration::TSInterfaceDeclaration(iface) => {
                self.exports.push(ExportInfo {
                    name: ExportName::Named(iface.id.name.to_string()),
                    local_name: Some(iface.id.name.to_string()),
                    is_type_only: true,
                    is_public: false,
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
                            has_decorator: false,
                        })
                    })
                    .collect();
                self.exports.push(ExportInfo {
                    name: ExportName::Named(enumd.id.name.to_string()),
                    local_name: Some(enumd.id.name.to_string()),
                    is_type_only,
                    is_public: false,
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
                        is_public: false,
                        span: id.span,
                        members: vec![],
                    });
                }
                TSModuleDeclarationName::StringLiteral(lit) => {
                    self.exports.push(ExportInfo {
                        name: ExportName::Named(lit.value.to_string()),
                        local_name: Some(lit.value.to_string()),
                        is_type_only: true,
                        is_public: false,
                        span: lit.span,
                        members: vec![],
                    });
                }
            },
            _ => {}
        }
    }

    pub(crate) fn extract_binding_pattern_names(
        &mut self,
        pattern: &BindingPattern<'_>,
        is_type_only: bool,
    ) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                self.exports.push(ExportInfo {
                    name: ExportName::Named(id.name.to_string()),
                    local_name: Some(id.name.to_string()),
                    is_type_only,
                    is_public: false,
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

    /// Handle `const x = require('./y')` patterns, recording the require call
    /// and tracking namespace bindings for later member access narrowing.
    pub(super) fn handle_require_declaration(
        &mut self,
        declarator: &VariableDeclarator<'_>,
        call: &CallExpression<'_>,
        source: &str,
    ) {
        match &declarator.id {
            BindingPattern::ObjectPattern(obj_pat) => {
                let names = extract_destructured_names(obj_pat);
                self.require_calls.push(RequireCallInfo {
                    source: source.to_string(),
                    span: call.span,
                    destructured_names: names,
                    local_name: None,
                });
                self.handled_require_spans.insert(call.span);
            }
            BindingPattern::BindingIdentifier(id) => {
                let local = id.name.to_string();
                self.namespace_binding_names.push(local.clone());
                self.require_calls.push(RequireCallInfo {
                    source: source.to_string(),
                    span: call.span,
                    destructured_names: Vec::new(),
                    local_name: Some(local),
                });
                self.handled_require_spans.insert(call.span);
            }
            _ => {}
        }
    }

    /// Handle namespace destructuring: `const { a, b } = ns` where `ns` is a namespace
    /// import, dynamic import namespace, or require namespace.
    /// Records member accesses so the graph can narrow which exports are used.
    pub(super) fn handle_namespace_destructuring(
        &mut self,
        declarator: &VariableDeclarator<'_>,
        ident_name: &str,
    ) {
        if let BindingPattern::ObjectPattern(obj_pat) = &declarator.id {
            if obj_pat.rest.is_some() {
                // Rest element captures remaining properties — mark as whole-object use
                self.whole_object_uses.push(ident_name.to_string());
            } else {
                for prop in &obj_pat.properties {
                    if let Some(name) = prop.key.static_name() {
                        self.member_accesses.push(MemberAccess {
                            object: ident_name.to_string(),
                            member: name.to_string(),
                        });
                    }
                }
            }
        }
    }

    /// Handle `const x = await import('./y')` and `const x = import('./y')` patterns,
    /// recording the dynamic import and tracking namespace bindings.
    pub(super) fn handle_dynamic_import_declaration(
        &mut self,
        declarator: &VariableDeclarator<'_>,
        import_expr: &ImportExpression<'_>,
        source: &str,
    ) {
        match &declarator.id {
            BindingPattern::ObjectPattern(obj_pat) => {
                let names = extract_destructured_names(obj_pat);
                self.dynamic_imports.push(DynamicImportInfo {
                    source: source.to_string(),
                    span: import_expr.span,
                    destructured_names: names,
                    local_name: None,
                });
                self.handled_import_spans.insert(import_expr.span);
            }
            BindingPattern::BindingIdentifier(id) => {
                let local = id.name.to_string();
                self.namespace_binding_names.push(local.clone());
                self.dynamic_imports.push(DynamicImportInfo {
                    source: source.to_string(),
                    span: import_expr.span,
                    destructured_names: Vec::new(),
                    local_name: Some(local),
                });
                self.handled_import_spans.insert(import_expr.span);
            }
            _ => {}
        }
    }
}
