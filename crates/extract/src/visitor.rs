//! Oxc AST visitor for extracting imports, exports, re-exports, and member accesses.

use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_span::Span;

use crate::suppress::Suppression;
use crate::{
    DynamicImportInfo, DynamicImportPattern, ExportInfo, ExportName, ImportInfo, ImportedName,
    MemberAccess, MemberInfo, MemberKind, ModuleInfo, ReExportInfo, RequireCallInfo,
};

/// Extract class members (methods and properties) from a class declaration.
pub(crate) fn extract_class_members(class: &Class<'_>) -> Vec<MemberInfo> {
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
    pub(crate) exports: Vec<ExportInfo>,
    pub(crate) imports: Vec<ImportInfo>,
    pub(crate) re_exports: Vec<ReExportInfo>,
    pub(crate) dynamic_imports: Vec<DynamicImportInfo>,
    pub(crate) dynamic_import_patterns: Vec<DynamicImportPattern>,
    pub(crate) require_calls: Vec<RequireCallInfo>,
    pub(crate) member_accesses: Vec<MemberAccess>,
    pub(crate) whole_object_uses: Vec<String>,
    pub(crate) has_cjs_exports: bool,
    /// Spans of `require()` calls already handled via destructured require detection.
    handled_require_spans: Vec<Span>,
    /// Spans of `import()` expressions already handled via variable declarator detection.
    handled_import_spans: Vec<Span>,
    /// Local names of namespace imports and namespace-like bindings
    /// (e.g., `import * as ns`, `const mod = require(...)`, `const mod = await import(...)`).
    /// Used to detect destructuring patterns like `const { a, b } = ns`.
    namespace_binding_names: Vec<String>,
}

impl ModuleInfoExtractor {
    pub(crate) const fn new() -> Self {
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
            namespace_binding_names: Vec::new(),
        }
    }

    /// Convert this extractor into a `ModuleInfo`, consuming its fields.
    pub(crate) fn into_module_info(
        self,
        file_id: fallow_types::discover::FileId,
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
            unused_import_bindings: Vec::new(),
            line_offsets: Vec::new(),
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
                        let local = s.local.name.to_string();
                        self.namespace_binding_names.push(local.clone());
                        self.imports.push(ImportInfo {
                            source: source.clone(),
                            imported_name: ImportedName::Namespace,
                            local_name: local,
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
            .map_or_else(|| "*".to_string(), |e| e.name().to_string());

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
                        let local = id.name.to_string();
                        self.namespace_binding_names.push(local.clone());
                        self.require_calls.push(RequireCallInfo {
                            source,
                            span: call.span,
                            destructured_names: Vec::new(),
                            local_name: Some(local),
                        });
                        self.handled_require_spans.push(call.span);
                    }
                    _ => {}
                }
                continue;
            }

            // Detect namespace destructuring: `const { a, b } = ns` where `ns`
            // is a namespace import, dynamic import namespace, or require namespace.
            // Generates member accesses so the graph can narrow which exports are used.
            //
            // Limitation: `namespace_binding_names` is scope-unaware. A local variable
            // that shadows an import with the same name (e.g., `const ns = {...}` inside
            // a function after `import * as ns`) would also match. This is consistent
            // with the existing `member_accesses` approach which is also flat/unscoped.
            // The effect is a false negative (unused export not reported), not a false
            // positive, so it errs on the side of safety.
            if let Expression::Identifier(ident) = init
                && self
                    .namespace_binding_names
                    .iter()
                    .any(|n| n == ident.name.as_str())
            {
                if let BindingPattern::ObjectPattern(obj_pat) = &declarator.id {
                    if obj_pat.rest.is_some() {
                        // Rest element captures remaining properties — mark as whole-object use
                        self.whole_object_uses.push(ident.name.to_string());
                    } else {
                        for prop in &obj_pat.properties {
                            if let Some(name) = prop.key.static_name() {
                                self.member_accesses.push(MemberAccess {
                                    object: ident.name.to_string(),
                                    member: name.to_string(),
                                });
                            }
                        }
                    }
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
                    let local = id.name.to_string();
                    self.namespace_binding_names.push(local.clone());
                    self.dynamic_imports.push(DynamicImportInfo {
                        source,
                        span: import_expr.span,
                        destructured_names: Vec::new(),
                        local_name: Some(local),
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
            && matches!(
                member.property.name.as_str(),
                "values" | "keys" | "entries" | "getOwnPropertyNames"
            )
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
                // Parse the optional third argument (regex filter) and convert
                // simple extension patterns (e.g., /\.vue$/) to a glob suffix.
                let suffix = expr.arguments.get(2).and_then(|arg| match arg {
                    Argument::RegExpLiteral(re) => regex_pattern_to_suffix(&re.regex.pattern.text),
                    _ => None,
                });
                self.dynamic_import_patterns.push(DynamicImportPattern {
                    prefix,
                    suffix,
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

/// Convert a simple regex extension filter pattern to a glob suffix.
///
/// Handles common `require.context()` patterns like:
/// - `\.vue$` → `".vue"`
/// - `\.tsx?$` → uses `".ts"` / `".tsx"` via glob `".{ts,tsx}"`
/// - `\.(js|ts)$` → `".{js,ts}"`
/// - `\.(js|jsx|ts|tsx)$` → `".{js,jsx,ts,tsx}"`
///
/// Returns `None` for patterns that are too complex to convert.
fn regex_pattern_to_suffix(pattern: &str) -> Option<String> {
    // Strip leading `^` or `.*` anchors (they don't affect extension matching)
    let p = pattern.strip_prefix('^').unwrap_or(pattern);
    let p = p.strip_prefix(".*").unwrap_or(p);

    // Must start with `\.` (escaped dot for extension)
    let p = p.strip_prefix("\\.")?;

    // Must end with `$`
    let p = p.strip_suffix('$')?;

    // Pattern: `ext?` — e.g., `tsx?` → {ts,tsx}
    if let Some(base) = p.strip_suffix('?') {
        // base must be simple alphanumeric (e.g., "tsx" from "tsx?")
        if base.chars().all(|c| c.is_ascii_alphanumeric()) && !base.is_empty() {
            let without_last = &base[..base.len() - 1];
            if without_last.is_empty() {
                // Single char like `x?` → matches "" or "x", too ambiguous
                return None;
            }
            return Some(format!(".{{{without_last},{base}}}"));
        }
        return None;
    }

    // Pattern: `(ext1|ext2|...)` — e.g., `(js|ts)` → {js,ts}
    if let Some(inner) = p.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        let exts: Vec<&str> = inner.split('|').collect();
        if exts
            .iter()
            .all(|e| e.chars().all(|c| c.is_ascii_alphanumeric()) && !e.is_empty())
        {
            return Some(format!(".{{{}}}", exts.join(",")));
        }
        return None;
    }

    // Pattern: simple extension like `vue`, `json`, `css`
    if p.chars().all(|c| c.is_ascii_alphanumeric()) && !p.is_empty() {
        return Some(format!(".{p}"));
    }

    None
}

// Visitor tests invoke Oxc parser which is ~1000x slower under Miri.
#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use crate::parse::parse_source_to_module;
    use fallow_types::discover::FileId;
    use std::path::Path;

    /// Helper: parse TypeScript source and return ModuleInfo.
    fn parse(source: &str) -> crate::ModuleInfo {
        parse_source_to_module(FileId(0), Path::new("test.ts"), source, 0)
    }

    // ── into_module_info transfers all fields ────────────────────

    #[test]
    fn into_module_info_transfers_exports() {
        let info = parse("export const a = 1; export function b() {}");
        assert_eq!(info.exports.len(), 2);
        assert_eq!(info.file_id, FileId(0));
    }

    #[test]
    fn into_module_info_transfers_imports() {
        let info = parse("import { foo } from './bar'; import baz from 'baz';");
        assert_eq!(info.imports.len(), 2);
    }

    #[test]
    fn into_module_info_transfers_re_exports() {
        let info = parse("export { foo } from './bar'; export * from './baz';");
        assert_eq!(info.re_exports.len(), 2);
    }

    #[test]
    fn into_module_info_transfers_dynamic_imports() {
        let info = parse("const m = import('./lazy');");
        assert_eq!(info.dynamic_imports.len(), 1);
    }

    #[test]
    fn into_module_info_transfers_require_calls() {
        let info = parse("const x = require('./util');");
        assert_eq!(info.require_calls.len(), 1);
    }

    #[test]
    fn into_module_info_transfers_whole_object_uses() {
        let info = parse(
            "import { Status } from './types';\nObject.values(Status);\nconst y = { ...Status };",
        );
        // Object.values + spread = 2 whole-object uses
        assert!(info.whole_object_uses.len() >= 2);
    }

    #[test]
    fn into_module_info_transfers_member_accesses() {
        let info = parse("import { Obj } from './x';\nObj.method();");
        assert!(
            info.member_accesses
                .iter()
                .any(|a| a.object == "Obj" && a.member == "method")
        );
    }

    #[test]
    fn into_module_info_transfers_cjs_flag() {
        let info = parse("module.exports = {};");
        assert!(info.has_cjs_exports);
    }

    // ── merge_into extends (not replaces) ────────────────────────

    #[test]
    fn merge_into_extends_imports() {
        let mut base = parse("import { a } from './a';");
        let _extra = parse("import { b } from './b';");

        // Build a second extractor from parsing and merge
        let allocator = oxc_allocator::Allocator::default();
        let source_type =
            oxc_span::SourceType::from_path(Path::new("extra.ts")).unwrap_or_default();
        let parser_return =
            oxc_parser::Parser::new(&allocator, "import { c } from './c';", source_type).parse();
        let mut extractor = ModuleInfoExtractor::new();
        oxc_ast_visit::Visit::visit_program(&mut extractor, &parser_return.program);
        extractor.merge_into(&mut base);

        assert!(
            base.imports.len() >= 2,
            "merge_into should add to existing imports, not replace"
        );
    }

    #[test]
    fn merge_into_ors_cjs_flag() {
        let mut base = parse("export const x = 1;");
        assert!(!base.has_cjs_exports);

        let allocator = oxc_allocator::Allocator::default();
        let source_type = oxc_span::SourceType::from_path(Path::new("cjs.ts")).unwrap_or_default();
        let parser_return =
            oxc_parser::Parser::new(&allocator, "module.exports = {};", source_type).parse();
        let mut extractor = ModuleInfoExtractor::new();
        oxc_ast_visit::Visit::visit_program(&mut extractor, &parser_return.program);
        extractor.merge_into(&mut base);

        assert!(base.has_cjs_exports, "merge_into should OR the cjs flag");
    }

    // ── Class member extraction ──────────────────────────────────

    #[test]
    fn extracts_public_class_methods_and_properties() {
        let info = parse(
            r#"
            export class MyService {
                name: string;
                getValue() { return 1; }
            }
            "#,
        );
        let class_export = info
            .exports
            .iter()
            .find(|e| matches!(&e.name, ExportName::Named(n) if n == "MyService"));
        assert!(class_export.is_some());
        let members = &class_export.unwrap().members;
        assert!(
            members
                .iter()
                .any(|m| m.name == "name" && m.kind == MemberKind::ClassProperty),
            "should extract 'name' property"
        );
        assert!(
            members
                .iter()
                .any(|m| m.name == "getValue" && m.kind == MemberKind::ClassMethod),
            "should extract 'getValue' method"
        );
    }

    #[test]
    fn skips_constructor_in_class_members() {
        let info = parse(
            r#"
            export class Foo {
                constructor() {}
                doWork() {}
            }
            "#,
        );
        let class_export = info
            .exports
            .iter()
            .find(|e| matches!(&e.name, ExportName::Named(n) if n == "Foo"));
        let members = &class_export.unwrap().members;
        assert!(
            !members.iter().any(|m| m.name == "constructor"),
            "constructor should be skipped"
        );
        assert!(members.iter().any(|m| m.name == "doWork"));
    }

    #[test]
    fn skips_private_and_protected_members() {
        let info = parse(
            r#"
            export class Foo {
                private secret: string;
                protected internal(): void {}
                public visible: number;
            }
            "#,
        );
        let class_export = info
            .exports
            .iter()
            .find(|e| matches!(&e.name, ExportName::Named(n) if n == "Foo"));
        let members = &class_export.unwrap().members;
        assert!(
            !members.iter().any(|m| m.name == "secret"),
            "private members should be skipped"
        );
        assert!(
            !members.iter().any(|m| m.name == "internal"),
            "protected members should be skipped"
        );
        assert!(
            members.iter().any(|m| m.name == "visible"),
            "public members should be included"
        );
    }

    #[test]
    fn class_member_with_decorator_flagged() {
        let info = parse(
            r#"
            function Injectable() { return (target: any) => target; }
            export class Service {
                @Injectable()
                handler() {}
            }
            "#,
        );
        let class_export = info
            .exports
            .iter()
            .find(|e| matches!(&e.name, ExportName::Named(n) if n == "Service"));
        let members = &class_export.unwrap().members;
        let handler = members.iter().find(|m| m.name == "handler");
        assert!(handler.is_some());
        assert!(
            handler.unwrap().has_decorator,
            "decorated member should have has_decorator = true"
        );
    }

    // ── Enum member extraction ───────────────────────────────────

    #[test]
    fn extracts_enum_members() {
        let info = parse(
            r#"
            export enum Direction {
                Up,
                Down,
                Left,
                Right
            }
            "#,
        );
        let enum_export = info
            .exports
            .iter()
            .find(|e| matches!(&e.name, ExportName::Named(n) if n == "Direction"));
        assert!(enum_export.is_some());
        let members = &enum_export.unwrap().members;
        assert_eq!(members.len(), 4);
        assert!(members.iter().all(|m| m.kind == MemberKind::EnumMember));
        assert!(members.iter().any(|m| m.name == "Up"));
        assert!(members.iter().any(|m| m.name == "Right"));
    }

    // ── Whole-object use patterns ────────────────────────────────

    #[test]
    fn object_values_marks_whole_use() {
        let info = parse("import { E } from './e';\nObject.values(E);");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    #[test]
    fn object_keys_marks_whole_use() {
        let info = parse("import { E } from './e';\nObject.keys(E);");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    #[test]
    fn object_entries_marks_whole_use() {
        let info = parse("import { E } from './e';\nObject.entries(E);");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    #[test]
    fn for_in_marks_whole_use() {
        let info = parse("import { E } from './e';\nfor (const k in E) {}");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    #[test]
    fn spread_marks_whole_use() {
        let info = parse("import { E } from './e';\nconst x = { ...E };");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    #[test]
    fn dynamic_computed_access_marks_whole_use() {
        let info = parse("import { E } from './e';\nconst k = 'x';\nE[k];");
        assert!(info.whole_object_uses.contains(&"E".to_string()));
    }

    // ── this.member tracking ─────────────────────────────────────

    #[test]
    fn this_member_access_tracked() {
        let info = parse(
            r#"
            export class Foo {
                bar: number;
                baz() { return this.bar; }
            }
            "#,
        );
        assert!(
            info.member_accesses
                .iter()
                .any(|a| a.object == "this" && a.member == "bar"),
            "this.bar should be tracked as a member access"
        );
    }

    #[test]
    fn this_assignment_tracked() {
        let info = parse(
            r#"
            export class Foo {
                bar: number;
                init() { this.bar = 42; }
            }
            "#,
        );
        assert!(
            info.member_accesses
                .iter()
                .any(|a| a.object == "this" && a.member == "bar"),
            "this.bar = ... should be tracked as a member access"
        );
    }

    // ── CJS export patterns ──────────────────────────────────────

    #[test]
    fn module_exports_object_extracts_keys() {
        let info = parse("module.exports = { foo: 1, bar: 2 };");
        assert!(info.has_cjs_exports);
        assert!(
            info.exports
                .iter()
                .any(|e| matches!(&e.name, ExportName::Named(n) if n == "foo"))
        );
        assert!(
            info.exports
                .iter()
                .any(|e| matches!(&e.name, ExportName::Named(n) if n == "bar"))
        );
    }

    #[test]
    fn exports_dot_property() {
        let info = parse("exports.myFunc = function() {};");
        assert!(info.has_cjs_exports);
        assert!(
            info.exports
                .iter()
                .any(|e| { matches!(&e.name, ExportName::Named(n) if n == "myFunc") })
        );
    }

    // ── Destructured require/import ──────────────────────────────

    #[test]
    fn destructured_require_captures_names() {
        let info = parse("const { readFile, writeFile } = require('fs');");
        assert_eq!(info.require_calls.len(), 1);
        let call = &info.require_calls[0];
        assert_eq!(call.source, "fs");
        assert!(call.destructured_names.contains(&"readFile".to_string()));
        assert!(call.destructured_names.contains(&"writeFile".to_string()));
    }

    #[test]
    fn namespace_require_has_local_name() {
        let info = parse("const fs = require('fs');");
        assert_eq!(info.require_calls.len(), 1);
        assert_eq!(info.require_calls[0].local_name, Some("fs".to_string()));
        assert!(info.require_calls[0].destructured_names.is_empty());
    }

    #[test]
    fn destructured_await_import_captures_names() {
        let info = parse("const { foo, bar } = await import('./mod');");
        assert_eq!(info.dynamic_imports.len(), 1);
        let imp = &info.dynamic_imports[0];
        assert_eq!(imp.source, "./mod");
        assert!(imp.destructured_names.contains(&"foo".to_string()));
        assert!(imp.destructured_names.contains(&"bar".to_string()));
    }

    #[test]
    fn namespace_await_import_has_local_name() {
        let info = parse("const mod = await import('./mod');");
        assert_eq!(info.dynamic_imports.len(), 1);
        assert_eq!(info.dynamic_imports[0].local_name, Some("mod".to_string()));
    }

    // ── new URL pattern ──────────────────────────────────────────

    #[test]
    fn new_url_with_import_meta_url_tracked() {
        let info = parse("const w = new URL('./worker.js', import.meta.url);");
        assert!(
            info.dynamic_imports
                .iter()
                .any(|d| d.source == "./worker.js"),
            "new URL('./worker.js', import.meta.url) should be tracked as dynamic import"
        );
    }

    // ── import.meta.glob ─────────────────────────────────────────

    #[test]
    fn import_meta_glob_string_pattern() {
        let info = parse("const mods = import.meta.glob('./modules/*.ts');");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./modules/*.ts");
    }

    #[test]
    fn import_meta_glob_array_patterns() {
        let info = parse("const mods = import.meta.glob(['./a/*.ts', './b/*.ts']);");
        assert_eq!(info.dynamic_import_patterns.len(), 2);
    }

    // ── require.context ──────────────────────────────────────────

    #[test]
    fn require_context_non_recursive() {
        let info = parse("const ctx = require.context('./components', false);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./components/");
    }

    #[test]
    fn require_context_recursive() {
        let info = parse("const ctx = require.context('./components', true);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./components/**/");
    }

    #[test]
    fn require_context_regex_simple_extension() {
        let info = parse("const ctx = require.context('./components', true, /\\.vue$/);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./components/**/");
        assert_eq!(
            info.dynamic_import_patterns[0].suffix,
            Some(".vue".to_string())
        );
    }

    #[test]
    fn require_context_regex_optional_char() {
        let info = parse("const ctx = require.context('./src', true, /\\.tsx?$/);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(
            info.dynamic_import_patterns[0].suffix,
            Some(".{ts,tsx}".to_string())
        );
    }

    #[test]
    fn require_context_regex_alternation() {
        let info = parse("const ctx = require.context('./src', false, /\\.(js|ts)$/);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert_eq!(info.dynamic_import_patterns[0].prefix, "./src/");
        assert_eq!(
            info.dynamic_import_patterns[0].suffix,
            Some(".{js,ts}".to_string())
        );
    }

    #[test]
    fn require_context_no_regex_has_no_suffix() {
        let info = parse("const ctx = require.context('./icons', true);");
        assert_eq!(info.dynamic_import_patterns.len(), 1);
        assert!(info.dynamic_import_patterns[0].suffix.is_none());
    }

    // ── regex_pattern_to_suffix unit tests ──────────────────────

    #[test]
    fn regex_suffix_simple_ext() {
        assert_eq!(regex_pattern_to_suffix(r"\.vue$"), Some(".vue".to_string()));
        assert_eq!(
            regex_pattern_to_suffix(r"\.json$"),
            Some(".json".to_string())
        );
        assert_eq!(regex_pattern_to_suffix(r"\.css$"), Some(".css".to_string()));
    }

    #[test]
    fn regex_suffix_optional_char() {
        assert_eq!(
            regex_pattern_to_suffix(r"\.tsx?$"),
            Some(".{ts,tsx}".to_string())
        );
        assert_eq!(
            regex_pattern_to_suffix(r"\.jsx?$"),
            Some(".{js,jsx}".to_string())
        );
    }

    #[test]
    fn regex_suffix_alternation() {
        assert_eq!(
            regex_pattern_to_suffix(r"\.(js|ts)$"),
            Some(".{js,ts}".to_string())
        );
        assert_eq!(
            regex_pattern_to_suffix(r"\.(js|jsx|ts|tsx)$"),
            Some(".{js,jsx,ts,tsx}".to_string())
        );
    }

    #[test]
    fn regex_suffix_complex_returns_none() {
        // Patterns too complex to convert
        assert_eq!(regex_pattern_to_suffix(r"\..*$"), None);
        assert_eq!(regex_pattern_to_suffix(r"\.[^.]+$"), None);
        assert_eq!(regex_pattern_to_suffix(r"test"), None);
    }

    // ── Whole-object-use edge cases ─────────────────────────────

    #[test]
    fn for_in_loop_marks_enum_as_whole_use() {
        let info = parse(
            "import { MyEnum } from './types';\nfor (const key in MyEnum) { console.log(key); }",
        );
        assert!(
            info.whole_object_uses.contains(&"MyEnum".to_string()),
            "for...in should mark MyEnum as whole-object-use"
        );
    }

    #[test]
    fn spread_in_object_marks_whole_use() {
        let info = parse("import { obj } from './data';\nconst copy = { ...obj };");
        assert!(
            info.whole_object_uses.contains(&"obj".to_string()),
            "spread in object literal should mark obj as whole-object-use"
        );
    }

    #[test]
    fn object_get_own_property_names_marks_whole_use() {
        let info = parse("import { MyEnum } from './types';\nObject.getOwnPropertyNames(MyEnum);");
        assert!(
            info.whole_object_uses.contains(&"MyEnum".to_string()),
            "Object.getOwnPropertyNames should mark MyEnum as whole-object-use"
        );
    }

    #[test]
    fn nested_member_access_only_tracks_object() {
        let info = parse("import { obj } from './data';\nconst val = obj.nested.prop;");
        // obj should be tracked as a member access, not as whole-object-use
        assert!(
            info.member_accesses
                .iter()
                .any(|a| a.object == "obj" && a.member == "nested"),
            "obj.nested should be tracked as a member access"
        );
        // obj should NOT be in whole_object_uses (it's a specific member access)
        assert!(
            !info.whole_object_uses.contains(&"obj".to_string()),
            "nested member access should not mark obj as whole-object-use"
        );
    }
}
