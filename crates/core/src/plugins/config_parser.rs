//! AST-based config file parser utilities.
//!
//! Provides helpers to extract configuration values from JS/TS config files
//! without evaluating them. Uses Oxc's parser for fast, safe AST walking.
//!
//! Common patterns handled:
//! - `export default { key: "value" }` (default export object)
//! - `export default defineConfig({ key: "value" })` (factory function)
//! - `module.exports = { key: "value" }` (CJS)
//! - Import specifiers (`import x from 'pkg'`)
//! - Array literals (`["a", "b"]`)
//! - Object properties (`{ key: "value" }`)

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Extract all import source specifiers from JS/TS source code.
pub fn extract_imports(source: &str, path: &Path) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let mut sources = Vec::new();
        for stmt in &program.body {
            if let Statement::ImportDeclaration(decl) = stmt {
                sources.push(decl.source.value.to_string());
            }
        }
        Some(sources)
    })
    .unwrap_or_default()
}

/// Extract string array from a property at a nested path in a config's default export.
pub fn extract_config_string_array(source: &str, path: &Path, prop_path: &[&str]) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        get_nested_string_array_from_object(obj, prop_path)
    })
    .unwrap_or_default()
}

/// Extract a single string from a property at a nested path.
pub fn extract_config_string(source: &str, path: &Path, prop_path: &[&str]) -> Option<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        get_nested_string_from_object(obj, prop_path)
    })
}

/// Extract string values from top-level properties of the default export/module.exports object.
/// Returns all string literal values found for the given property key, recursively.
///
/// **Warning**: This recurses into nested objects/arrays. For config arrays that contain
/// tuples like `["pkg-name", { options }]`, use [`extract_config_shallow_strings`] instead
/// to avoid extracting option values as package names.
pub fn extract_config_property_strings(source: &str, path: &Path, key: &str) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        let mut values = Vec::new();
        if let Some(prop) = find_property(obj, key) {
            collect_all_string_values(&prop.value, &mut values);
        }
        Some(values)
    })
    .unwrap_or_default()
}

/// Extract only top-level string values from a property's array.
///
/// Unlike [`extract_config_property_strings`], this does NOT recurse into nested
/// objects or sub-arrays. Useful for config arrays with tuple elements like:
/// `reporters: ["default", ["jest-junit", { outputDirectory: "reports" }]]`
/// — only `"default"` and `"jest-junit"` are returned, not `"reports"`.
pub fn extract_config_shallow_strings(source: &str, path: &Path, key: &str) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        let prop = find_property(obj, key)?;
        Some(collect_shallow_string_values(&prop.value))
    })
    .unwrap_or_default()
}

/// Public wrapper for `find_config_object` for plugins that need manual AST walking.
pub fn find_config_object_pub<'a>(program: &'a Program) -> Option<&'a ObjectExpression<'a>> {
    find_config_object(program)
}

/// Extract keys of an object property at a nested path.
///
/// Useful for `PostCSS` config: `{ plugins: { autoprefixer: {}, tailwindcss: {} } }`
/// → returns `["autoprefixer", "tailwindcss"]`.
pub fn extract_config_object_keys(source: &str, path: &Path, prop_path: &[&str]) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        get_nested_object_keys(obj, prop_path)
    })
    .unwrap_or_default()
}

/// Extract a value that may be a single string, a string array, or an object with string values.
///
/// Useful for Webpack `entry`, Rollup `input`, etc. that accept multiple formats:
/// - `entry: "./src/index.js"` → `["./src/index.js"]`
/// - `entry: ["./src/a.js", "./src/b.js"]` → `["./src/a.js", "./src/b.js"]`
/// - `entry: { main: "./src/main.js" }` → `["./src/main.js"]`
pub fn extract_config_string_or_array(
    source: &str,
    path: &Path,
    prop_path: &[&str],
) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        get_nested_string_or_array(obj, prop_path)
    })
    .unwrap_or_default()
}

/// Extract string values from a property path, also searching inside array elements.
///
/// Navigates `array_path` to find an array expression, then for each object in the
/// array, navigates `inner_path` to extract string values. Useful for configs like
/// Vitest projects where values are nested in array elements:
/// - `test.projects[*].test.setupFiles`
pub fn extract_config_array_nested_string_or_array(
    source: &str,
    path: &Path,
    array_path: &[&str],
    inner_path: &[&str],
) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        let array_expr = get_nested_expression(obj, array_path)?;
        let Expression::ArrayExpression(arr) = array_expr else {
            return None;
        };
        let mut results = Vec::new();
        for element in &arr.elements {
            if let Some(Expression::ObjectExpression(element_obj)) = element.as_expression()
                && let Some(values) = get_nested_string_or_array(element_obj, inner_path)
            {
                results.extend(values);
            }
        }
        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    })
    .unwrap_or_default()
}

/// Extract string values from a property path, searching inside all values of an object.
///
/// Navigates `object_path` to find an object expression, then for each property value
/// (regardless of key name), navigates `inner_path` to extract string values. Useful for
/// configs with dynamic keys like `angular.json`:
/// - `projects.*.architect.build.options.styles`
pub fn extract_config_object_nested_string_or_array(
    source: &str,
    path: &Path,
    object_path: &[&str],
    inner_path: &[&str],
) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        let obj_expr = get_nested_expression(obj, object_path)?;
        let Expression::ObjectExpression(target_obj) = obj_expr else {
            return None;
        };
        let mut results = Vec::new();
        for prop in &target_obj.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop
                && let Expression::ObjectExpression(value_obj) = &p.value
                && let Some(values) = get_nested_string_or_array(value_obj, inner_path)
            {
                results.extend(values);
            }
        }
        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    })
    .unwrap_or_default()
}

/// Extract string values from a property path, searching inside all values of an object.
///
/// Like [`extract_config_object_nested_string_or_array`] but returns a single optional string
/// per object value (useful for fields like `architect.build.options.main`).
pub fn extract_config_object_nested_strings(
    source: &str,
    path: &Path,
    object_path: &[&str],
    inner_path: &[&str],
) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        let obj_expr = get_nested_expression(obj, object_path)?;
        let Expression::ObjectExpression(target_obj) = obj_expr else {
            return None;
        };
        let mut results = Vec::new();
        for prop in &target_obj.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop
                && let Expression::ObjectExpression(value_obj) = &p.value
                && let Some(value) = get_nested_string_from_object(value_obj, inner_path)
            {
                results.push(value);
            }
        }
        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    })
    .unwrap_or_default()
}

/// Extract `require('...')` call argument strings from a property's value.
///
/// Handles direct require calls and arrays containing require calls or tuples:
/// - `plugins: [require('autoprefixer')]`
/// - `plugins: [require('postcss-import'), [require('postcss-preset-env'), { ... }]]`
pub fn extract_config_require_strings(source: &str, path: &Path, key: &str) -> Vec<String> {
    extract_from_source(source, path, |program| {
        let obj = find_config_object(program)?;
        let prop = find_property(obj, key)?;
        Some(collect_require_sources(&prop.value))
    })
    .unwrap_or_default()
}

// ── Internal helpers ──────────────────────────────────────────────

/// Parse source and run an extraction function on the AST.
///
/// JSON files (`.json`, `.jsonc`) are parsed as JavaScript expressions wrapped in
/// parentheses to produce an AST compatible with `find_config_object`. The native
/// JSON source type in Oxc produces a different AST structure that our helpers
/// don't handle.
fn extract_from_source<T>(
    source: &str,
    path: &Path,
    extractor: impl FnOnce(&Program) -> Option<T>,
) -> Option<T> {
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let alloc = Allocator::default();

    // For JSON files, wrap in parens and parse as JS so the AST matches
    // what find_config_object expects (ExpressionStatement → ObjectExpression).
    let is_json = path
        .extension()
        .is_some_and(|ext| ext == "json" || ext == "jsonc");
    if is_json {
        let wrapped = format!("({source})");
        let parsed = Parser::new(&alloc, &wrapped, SourceType::mjs()).parse();
        return extractor(&parsed.program);
    }

    let parsed = Parser::new(&alloc, source, source_type).parse();
    extractor(&parsed.program)
}

/// Find the "config object" — the object expression in the default export or module.exports.
///
/// Handles these patterns:
/// - `export default { ... }`
/// - `export default defineConfig({ ... })`
/// - `export default defineConfig(async () => ({ ... }))`
/// - `module.exports = { ... }`
/// - Top-level JSON object (for .json files)
fn find_config_object<'a>(program: &'a Program) -> Option<&'a ObjectExpression<'a>> {
    for stmt in &program.body {
        match stmt {
            // export default { ... } or export default defineConfig({ ... })
            Statement::ExportDefaultDeclaration(decl) => {
                // ExportDefaultDeclarationKind inherits Expression variants directly
                let expr: Option<&Expression> = match &decl.declaration {
                    ExportDefaultDeclarationKind::ObjectExpression(obj) => {
                        return Some(obj);
                    }
                    ExportDefaultDeclarationKind::CallExpression(_)
                    | ExportDefaultDeclarationKind::ParenthesizedExpression(_) => {
                        // Convert to expression reference for further extraction
                        decl.declaration.as_expression()
                    }
                    _ => None,
                };
                if let Some(expr) = expr {
                    return extract_object_from_expression(expr);
                }
            }
            // module.exports = { ... }
            Statement::ExpressionStatement(expr_stmt) => {
                if let Expression::AssignmentExpression(assign) = &expr_stmt.expression
                    && is_module_exports_target(&assign.left)
                {
                    return extract_object_from_expression(&assign.right);
                }
            }
            _ => {}
        }
    }

    // JSON files: the program body might be a single expression statement
    // Also handles JSON wrapped in parens: `({ ... })` (used for tsconfig.json parsing)
    if program.body.len() == 1
        && let Statement::ExpressionStatement(expr_stmt) = &program.body[0]
    {
        match &expr_stmt.expression {
            Expression::ObjectExpression(obj) => return Some(obj),
            Expression::ParenthesizedExpression(paren) => {
                if let Expression::ObjectExpression(obj) = &paren.expression {
                    return Some(obj);
                }
            }
            _ => {}
        }
    }

    None
}

/// Extract an `ObjectExpression` from an expression, handling wrapper patterns.
fn extract_object_from_expression<'a>(
    expr: &'a Expression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match expr {
        // Direct object: `{ ... }`
        Expression::ObjectExpression(obj) => Some(obj),
        // Factory call: `defineConfig({ ... })`
        Expression::CallExpression(call) => {
            // Look for the first object argument
            for arg in &call.arguments {
                match arg {
                    Argument::ObjectExpression(obj) => return Some(obj),
                    // Arrow function body: `defineConfig(() => ({ ... }))`
                    Argument::ArrowFunctionExpression(arrow) => {
                        if arrow.expression
                            && !arrow.body.statements.is_empty()
                            && let Statement::ExpressionStatement(expr_stmt) =
                                &arrow.body.statements[0]
                        {
                            return extract_object_from_expression(&expr_stmt.expression);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        // Parenthesized: `({ ... })`
        Expression::ParenthesizedExpression(paren) => {
            extract_object_from_expression(&paren.expression)
        }
        _ => None,
    }
}

/// Check if an assignment target is `module.exports`.
fn is_module_exports_target(target: &AssignmentTarget) -> bool {
    if let AssignmentTarget::StaticMemberExpression(member) = target
        && let Expression::Identifier(obj) = &member.object
    {
        return obj.name == "module" && member.property.name == "exports";
    }
    false
}

/// Find a named property in an object expression.
fn find_property<'a>(obj: &'a ObjectExpression<'a>, key: &str) -> Option<&'a ObjectProperty<'a>> {
    for prop in &obj.properties {
        if let ObjectPropertyKind::ObjectProperty(p) = prop
            && property_key_matches(&p.key, key)
        {
            return Some(p);
        }
    }
    None
}

/// Check if a property key matches a string.
fn property_key_matches(key: &PropertyKey, name: &str) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name == name,
        PropertyKey::StringLiteral(s) => s.value == name,
        _ => false,
    }
}

/// Get a string value from an object property.
fn get_object_string_property(obj: &ObjectExpression, key: &str) -> Option<String> {
    find_property(obj, key).and_then(|p| expression_to_string(&p.value))
}

/// Get an array of strings from an object property.
fn get_object_string_array_property(obj: &ObjectExpression, key: &str) -> Vec<String> {
    find_property(obj, key)
        .map(|p| expression_to_string_array(&p.value))
        .unwrap_or_default()
}

/// Navigate a nested property path and get a string array.
fn get_nested_string_array_from_object(
    obj: &ObjectExpression,
    path: &[&str],
) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return Some(get_object_string_array_property(obj, path[0]));
    }
    // Navigate into nested object
    let prop = find_property(obj, path[0])?;
    if let Expression::ObjectExpression(nested) = &prop.value {
        get_nested_string_array_from_object(nested, &path[1..])
    } else {
        None
    }
}

/// Navigate a nested property path and get a string value.
fn get_nested_string_from_object(obj: &ObjectExpression, path: &[&str]) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return get_object_string_property(obj, path[0]);
    }
    let prop = find_property(obj, path[0])?;
    if let Expression::ObjectExpression(nested) = &prop.value {
        get_nested_string_from_object(nested, &path[1..])
    } else {
        None
    }
}

/// Convert an expression to a string if it's a string literal.
fn expression_to_string(expr: &Expression) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.to_string()),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => {
            // Template literal with no expressions: `\`value\``
            t.quasis.first().map(|q| q.value.raw.to_string())
        }
        _ => None,
    }
}

/// Convert an expression to a string array if it's an array of string literals.
fn expression_to_string_array(expr: &Expression) -> Vec<String> {
    match expr {
        Expression::ArrayExpression(arr) => arr
            .elements
            .iter()
            .filter_map(|el| match el {
                ArrayExpressionElement::SpreadElement(_) => None,
                _ => el.as_expression().and_then(expression_to_string),
            })
            .collect(),
        _ => vec![],
    }
}

/// Collect only top-level string values from an expression.
///
/// For arrays, extracts direct string elements and the first string element of sub-arrays
/// (to handle `["pkg-name", { options }]` tuples). Does NOT recurse into objects.
fn collect_shallow_string_values(expr: &Expression) -> Vec<String> {
    let mut values = Vec::new();
    match expr {
        Expression::StringLiteral(s) => {
            values.push(s.value.to_string());
        }
        Expression::ArrayExpression(arr) => {
            for el in &arr.elements {
                if let Some(inner) = el.as_expression() {
                    match inner {
                        Expression::StringLiteral(s) => {
                            values.push(s.value.to_string());
                        }
                        // Handle tuples: ["pkg-name", { options }] → extract first string
                        Expression::ArrayExpression(sub_arr) => {
                            if let Some(first) = sub_arr.elements.first()
                                && let Some(first_expr) = first.as_expression()
                                && let Some(s) = expression_to_string(first_expr)
                            {
                                values.push(s);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
    values
}

/// Recursively collect all string literal values from an expression tree.
fn collect_all_string_values(expr: &Expression, values: &mut Vec<String>) {
    match expr {
        Expression::StringLiteral(s) => {
            values.push(s.value.to_string());
        }
        Expression::ArrayExpression(arr) => {
            for el in &arr.elements {
                if let Some(expr) = el.as_expression() {
                    collect_all_string_values(expr, values);
                }
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    collect_all_string_values(&p.value, values);
                }
            }
        }
        _ => {}
    }
}

/// Convert a `PropertyKey` to a `String`.
fn property_key_to_string(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
        _ => None,
    }
}

/// Extract keys of an object at a nested property path.
fn get_nested_object_keys(obj: &ObjectExpression, path: &[&str]) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }
    let prop = find_property(obj, path[0])?;
    if path.len() == 1 {
        if let Expression::ObjectExpression(nested) = &prop.value {
            let keys = nested
                .properties
                .iter()
                .filter_map(|p| {
                    if let ObjectPropertyKind::ObjectProperty(p) = p {
                        property_key_to_string(&p.key)
                    } else {
                        None
                    }
                })
                .collect();
            return Some(keys);
        }
        return None;
    }
    if let Expression::ObjectExpression(nested) = &prop.value {
        get_nested_object_keys(nested, &path[1..])
    } else {
        None
    }
}

/// Navigate a nested property path and return the raw expression at the end.
fn get_nested_expression<'a>(
    obj: &'a ObjectExpression<'a>,
    path: &[&str],
) -> Option<&'a Expression<'a>> {
    if path.is_empty() {
        return None;
    }
    let prop = find_property(obj, path[0])?;
    if path.len() == 1 {
        return Some(&prop.value);
    }
    if let Expression::ObjectExpression(nested) = &prop.value {
        get_nested_expression(nested, &path[1..])
    } else {
        None
    }
}

/// Navigate a nested path and extract a string, string array, or object string values.
fn get_nested_string_or_array(obj: &ObjectExpression, path: &[&str]) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        let prop = find_property(obj, path[0])?;
        return Some(expression_to_string_or_array(&prop.value));
    }
    let prop = find_property(obj, path[0])?;
    if let Expression::ObjectExpression(nested) = &prop.value {
        get_nested_string_or_array(nested, &path[1..])
    } else {
        None
    }
}

/// Convert an expression to a `Vec<String>`, handling string, array, and object-with-string-values.
fn expression_to_string_or_array(expr: &Expression) -> Vec<String> {
    match expr {
        Expression::StringLiteral(s) => vec![s.value.to_string()],
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => t
            .quasis
            .first()
            .map(|q| vec![q.value.raw.to_string()])
            .unwrap_or_default(),
        Expression::ArrayExpression(arr) => arr
            .elements
            .iter()
            .filter_map(|el| el.as_expression().and_then(expression_to_string))
            .collect(),
        Expression::ObjectExpression(obj) => obj
            .properties
            .iter()
            .filter_map(|p| {
                if let ObjectPropertyKind::ObjectProperty(p) = p {
                    expression_to_string(&p.value)
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// Collect `require('...')` argument strings from an expression.
fn collect_require_sources(expr: &Expression) -> Vec<String> {
    let mut sources = Vec::new();
    match expr {
        Expression::CallExpression(call) if is_require_call(call) => {
            if let Some(s) = get_require_source(call) {
                sources.push(s);
            }
        }
        Expression::ArrayExpression(arr) => {
            for el in &arr.elements {
                if let Some(inner) = el.as_expression() {
                    match inner {
                        Expression::CallExpression(call) if is_require_call(call) => {
                            if let Some(s) = get_require_source(call) {
                                sources.push(s);
                            }
                        }
                        // Tuple: [require('pkg'), options]
                        Expression::ArrayExpression(sub_arr) => {
                            if let Some(first) = sub_arr.elements.first()
                                && let Some(Expression::CallExpression(call)) =
                                    first.as_expression()
                                && is_require_call(call)
                                && let Some(s) = get_require_source(call)
                            {
                                sources.push(s);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
    sources
}

/// Check if a call expression is `require(...)`.
fn is_require_call(call: &CallExpression) -> bool {
    matches!(&call.callee, Expression::Identifier(id) if id.name == "require")
}

/// Get the first string argument of a `require()` call.
fn get_require_source(call: &CallExpression) -> Option<String> {
    call.arguments.first().and_then(|arg| {
        if let Argument::StringLiteral(s) = arg {
            Some(s.value.to_string())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn js_path() -> PathBuf {
        PathBuf::from("config.js")
    }

    fn ts_path() -> PathBuf {
        PathBuf::from("config.ts")
    }

    #[test]
    fn extract_imports_basic() {
        let source = r#"
            import foo from 'foo-pkg';
            import { bar } from '@scope/bar';
            export default {};
        "#;
        let imports = extract_imports(source, &js_path());
        assert_eq!(imports, vec!["foo-pkg", "@scope/bar"]);
    }

    #[test]
    fn extract_default_export_object_property() {
        let source = r#"export default { testDir: "./tests" };"#;
        let val = extract_config_string(source, &js_path(), &["testDir"]);
        assert_eq!(val, Some("./tests".to_string()));
    }

    #[test]
    fn extract_define_config_property() {
        let source = r#"
            import { defineConfig } from 'vitest/config';
            export default defineConfig({
                test: {
                    include: ["**/*.test.ts", "**/*.spec.ts"],
                    setupFiles: ["./test/setup.ts"]
                }
            });
        "#;
        let include = extract_config_string_array(source, &ts_path(), &["test", "include"]);
        assert_eq!(include, vec!["**/*.test.ts", "**/*.spec.ts"]);

        let setup = extract_config_string_array(source, &ts_path(), &["test", "setupFiles"]);
        assert_eq!(setup, vec!["./test/setup.ts"]);
    }

    #[test]
    fn extract_module_exports_property() {
        let source = r#"module.exports = { testEnvironment: "jsdom" };"#;
        let val = extract_config_string(source, &js_path(), &["testEnvironment"]);
        assert_eq!(val, Some("jsdom".to_string()));
    }

    #[test]
    fn extract_nested_string_array() {
        let source = r#"
            export default {
                resolve: {
                    alias: {
                        "@": "./src"
                    }
                },
                test: {
                    include: ["src/**/*.test.ts"]
                }
            };
        "#;
        let include = extract_config_string_array(source, &js_path(), &["test", "include"]);
        assert_eq!(include, vec!["src/**/*.test.ts"]);
    }

    #[test]
    fn extract_addons_array() {
        let source = r#"
            export default {
                addons: [
                    "@storybook/addon-a11y",
                    "@storybook/addon-docs",
                    "@storybook/addon-links"
                ]
            };
        "#;
        let addons = extract_config_property_strings(source, &ts_path(), "addons");
        assert_eq!(
            addons,
            vec![
                "@storybook/addon-a11y",
                "@storybook/addon-docs",
                "@storybook/addon-links"
            ]
        );
    }

    #[test]
    fn handle_empty_config() {
        let source = "";
        let result = extract_config_string(source, &js_path(), &["key"]);
        assert_eq!(result, None);
    }

    // ── extract_config_object_keys tests ────────────────────────────

    #[test]
    fn object_keys_postcss_plugins() {
        let source = r#"
            module.exports = {
                plugins: {
                    autoprefixer: {},
                    tailwindcss: {},
                    'postcss-import': {}
                }
            };
        "#;
        let keys = extract_config_object_keys(source, &js_path(), &["plugins"]);
        assert_eq!(keys, vec!["autoprefixer", "tailwindcss", "postcss-import"]);
    }

    #[test]
    fn object_keys_nested_path() {
        let source = r#"
            export default {
                build: {
                    plugins: {
                        minify: {},
                        compress: {}
                    }
                }
            };
        "#;
        let keys = extract_config_object_keys(source, &js_path(), &["build", "plugins"]);
        assert_eq!(keys, vec!["minify", "compress"]);
    }

    #[test]
    fn object_keys_empty_object() {
        let source = r#"export default { plugins: {} };"#;
        let keys = extract_config_object_keys(source, &js_path(), &["plugins"]);
        assert!(keys.is_empty());
    }

    #[test]
    fn object_keys_non_object_returns_empty() {
        let source = r#"export default { plugins: ["a", "b"] };"#;
        let keys = extract_config_object_keys(source, &js_path(), &["plugins"]);
        assert!(keys.is_empty());
    }

    // ── extract_config_string_or_array tests ────────────────────────

    #[test]
    fn string_or_array_single_string() {
        let source = r#"export default { entry: "./src/index.js" };"#;
        let result = extract_config_string_or_array(source, &js_path(), &["entry"]);
        assert_eq!(result, vec!["./src/index.js"]);
    }

    #[test]
    fn string_or_array_array() {
        let source = r#"export default { entry: ["./src/a.js", "./src/b.js"] };"#;
        let result = extract_config_string_or_array(source, &js_path(), &["entry"]);
        assert_eq!(result, vec!["./src/a.js", "./src/b.js"]);
    }

    #[test]
    fn string_or_array_object_values() {
        let source =
            r#"export default { entry: { main: "./src/main.js", vendor: "./src/vendor.js" } };"#;
        let result = extract_config_string_or_array(source, &js_path(), &["entry"]);
        assert_eq!(result, vec!["./src/main.js", "./src/vendor.js"]);
    }

    #[test]
    fn string_or_array_nested_path() {
        let source = r#"
            export default {
                build: {
                    rollupOptions: {
                        input: ["./index.html", "./about.html"]
                    }
                }
            };
        "#;
        let result = extract_config_string_or_array(
            source,
            &js_path(),
            &["build", "rollupOptions", "input"],
        );
        assert_eq!(result, vec!["./index.html", "./about.html"]);
    }

    #[test]
    fn string_or_array_template_literal() {
        let source = r#"export default { entry: `./src/index.js` };"#;
        let result = extract_config_string_or_array(source, &js_path(), &["entry"]);
        assert_eq!(result, vec!["./src/index.js"]);
    }

    // ── extract_config_require_strings tests ────────────────────────

    #[test]
    fn require_strings_array() {
        let source = r#"
            module.exports = {
                plugins: [
                    require('autoprefixer'),
                    require('postcss-import')
                ]
            };
        "#;
        let deps = extract_config_require_strings(source, &js_path(), "plugins");
        assert_eq!(deps, vec!["autoprefixer", "postcss-import"]);
    }

    #[test]
    fn require_strings_with_tuples() {
        let source = r#"
            module.exports = {
                plugins: [
                    require('autoprefixer'),
                    [require('postcss-preset-env'), { stage: 3 }]
                ]
            };
        "#;
        let deps = extract_config_require_strings(source, &js_path(), "plugins");
        assert_eq!(deps, vec!["autoprefixer", "postcss-preset-env"]);
    }

    #[test]
    fn require_strings_empty_array() {
        let source = r#"module.exports = { plugins: [] };"#;
        let deps = extract_config_require_strings(source, &js_path(), "plugins");
        assert!(deps.is_empty());
    }

    #[test]
    fn require_strings_no_require_calls() {
        let source = r#"module.exports = { plugins: ["a", "b"] };"#;
        let deps = extract_config_require_strings(source, &js_path(), "plugins");
        assert!(deps.is_empty());
    }

    // ── JSON wrapped in parens (for tsconfig.json parsing) ──────────

    #[test]
    fn json_wrapped_in_parens_string() {
        let source = r#"({"extends": "@tsconfig/node18/tsconfig.json"})"#;
        let val = extract_config_string(source, &js_path(), &["extends"]);
        assert_eq!(val, Some("@tsconfig/node18/tsconfig.json".to_string()));
    }

    #[test]
    fn json_wrapped_in_parens_nested_array() {
        let source =
            r#"({"compilerOptions": {"types": ["node", "jest"]}, "include": ["src/**/*"]})"#;
        let types = extract_config_string_array(source, &js_path(), &["compilerOptions", "types"]);
        assert_eq!(types, vec!["node", "jest"]);

        let include = extract_config_string_array(source, &js_path(), &["include"]);
        assert_eq!(include, vec!["src/**/*"]);
    }

    #[test]
    fn json_wrapped_in_parens_object_keys() {
        let source = r#"({"plugins": {"autoprefixer": {}, "tailwindcss": {}}})"#;
        let keys = extract_config_object_keys(source, &js_path(), &["plugins"]);
        assert_eq!(keys, vec!["autoprefixer", "tailwindcss"]);
    }
}
