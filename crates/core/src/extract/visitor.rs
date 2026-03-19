use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_span::Span;

use super::{
    DynamicImportInfo, DynamicImportPattern, ExportInfo, ExportName, ImportInfo, ImportedName,
    MemberAccess, MemberInfo, MemberKind, ModuleInfo, ReExportInfo, RequireCallInfo,
};
use crate::suppress::Suppression;

/// Extract class members (methods and properties) from a class declaration.
pub(super) fn extract_class_members(class: &Class<'_>) -> Vec<MemberInfo> {
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
                            has_decorator: !method.decorators.is_empty(),
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
                        has_decorator: !prop.decorators.is_empty(),
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
pub(crate) struct ModuleInfoExtractor {
    pub(super) exports: Vec<ExportInfo>,
    pub(super) imports: Vec<ImportInfo>,
    pub(super) re_exports: Vec<ReExportInfo>,
    pub(super) dynamic_imports: Vec<DynamicImportInfo>,
    pub(super) dynamic_import_patterns: Vec<DynamicImportPattern>,
    pub(super) require_calls: Vec<RequireCallInfo>,
    pub(super) member_accesses: Vec<MemberAccess>,
    pub(super) whole_object_uses: Vec<String>,
    pub(super) has_cjs_exports: bool,
    /// Spans of require() calls already handled via destructured require detection.
    handled_require_spans: Vec<Span>,
    /// Spans of import() expressions already handled via variable declarator detection.
    handled_import_spans: Vec<Span>,
}

impl ModuleInfoExtractor {
    pub(crate) fn new() -> Self {
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
            handled_import_spans: Vec::new(),
        }
    }

    /// Convert this extractor into a `ModuleInfo`, consuming its fields.
    pub(crate) fn into_module_info(
        self,
        file_id: crate::discover::FileId,
        content_hash: u64,
        suppressions: Vec<Suppression>,
    ) -> ModuleInfo {
        ModuleInfo {
            file_id,
            exports: self.exports,
            imports: self.imports,
            re_exports: self.re_exports,
            dynamic_imports: self.dynamic_imports,
            dynamic_import_patterns: self.dynamic_import_patterns,
            require_calls: self.require_calls,
            member_accesses: self.member_accesses,
            whole_object_uses: self.whole_object_uses,
            has_cjs_exports: self.has_cjs_exports,
            content_hash,
            suppressions,
        }
    }

    /// Merge this extractor's fields into an existing `ModuleInfo`.
    pub(crate) fn merge_into(self, info: &mut ModuleInfo) {
        info.imports.extend(self.imports);
        info.exports.extend(self.exports);
        info.re_exports.extend(self.re_exports);
        info.dynamic_imports.extend(self.dynamic_imports);
        info.dynamic_import_patterns
            .extend(self.dynamic_import_patterns);
        info.require_calls.extend(self.require_calls);
        info.member_accesses.extend(self.member_accesses);
        info.whole_object_uses.extend(self.whole_object_uses);
        info.has_cjs_exports |= self.has_cjs_exports;
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
                            has_decorator: false,
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
        // Skip imports already handled via visit_variable_declaration (with local_name capture)
        if self.handled_import_spans.contains(&expr.span) {
            walk::walk_import_expression(self, expr);
            return;
        }

        match &expr.source {
            Expression::StringLiteral(lit) => {
                self.dynamic_imports.push(DynamicImportInfo {
                    source: lit.value.to_string(),
                    span: expr.span,
                    destructured_names: Vec::new(),
                    local_name: None,
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
                        destructured_names: Vec::new(),
                        local_name: None,
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
            let Some(init) = &declarator.init else {
                continue;
            };

            // Try to detect `const x = require('./y')` patterns
            if let Expression::CallExpression(call) = init
                && let Expression::Identifier(callee) = &call.callee
                && callee.name == "require"
                && let Some(Argument::StringLiteral(lit)) = call.arguments.first()
            {
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
                continue;
            }

            // Try to detect `const x = await import('./y')` and `const x = import('./y')` patterns
            // The import expression may be wrapped in an AwaitExpression or used directly.
            let import_expr = match init {
                Expression::AwaitExpression(await_expr) => {
                    if let Expression::ImportExpression(imp) = &await_expr.argument {
                        Some(imp)
                    } else {
                        None
                    }
                }
                Expression::ImportExpression(imp) => Some(imp),
                _ => None,
            };

            let Some(import_expr) = import_expr else {
                continue;
            };

            let Expression::StringLiteral(lit) = &import_expr.source else {
                continue;
            };

            let source = lit.value.to_string();

            match &declarator.id {
                BindingPattern::ObjectPattern(obj_pat) => {
                    // `const { foo, bar } = await import('./x')` → Named imports
                    if obj_pat.rest.is_some() {
                        // Has rest element: conservative, treat as namespace
                        self.dynamic_imports.push(DynamicImportInfo {
                            source,
                            span: import_expr.span,
                            destructured_names: Vec::new(),
                            local_name: None,
                        });
                    } else {
                        let names: Vec<String> = obj_pat
                            .properties
                            .iter()
                            .filter_map(|prop| prop.key.static_name().map(|n| n.to_string()))
                            .collect();
                        self.dynamic_imports.push(DynamicImportInfo {
                            source,
                            span: import_expr.span,
                            destructured_names: names,
                            local_name: None,
                        });
                    }
                    self.handled_import_spans.push(import_expr.span);
                }
                BindingPattern::BindingIdentifier(id) => {
                    // `const mod = await import('./x')` → Namespace with local_name for narrowing
                    self.dynamic_imports.push(DynamicImportInfo {
                        source,
                        span: import_expr.span,
                        destructured_names: Vec::new(),
                        local_name: Some(id.name.to_string()),
                    });
                    self.handled_import_spans.push(import_expr.span);
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
                destructured_names: Vec::new(),
                local_name: None,
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
