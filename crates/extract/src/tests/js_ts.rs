use std::path::Path;

use fallow_types::discover::FileId;
use fallow_types::extract::{ExportName, ImportedName, MemberKind, ModuleInfo};

use crate::parse::parse_source_to_module;

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

// -- Whole-object use detection --

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

// -- Dynamic import pattern extraction --

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
    let info = parse_source("const m = import(`./plugins/${cat}/${name}.js`);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./plugins/**/");
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".js".to_string())
    );
}

// -- import.meta.glob / require.context --

#[test]
fn extracts_import_meta_glob_pattern() {
    let info = parse_source("const mods = import.meta.glob('./components/*.tsx');");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./components/*.tsx");
}

#[test]
fn extracts_import_meta_glob_array() {
    let info = parse_source("const mods = import.meta.glob(['./pages/*.ts', './layouts/*.ts']);");
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

// -- Dynamic import namespace tracking --

#[test]
fn dynamic_import_await_captures_local_name() {
    let info = parse_source(
        "async function f() { const mod = await import('./service'); mod.doStuff(); }",
    );
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./service");
    assert_eq!(info.dynamic_imports[0].local_name, Some("mod".to_string()));
    assert!(info.dynamic_imports[0].destructured_names.is_empty());
}

#[test]
fn dynamic_import_without_await_captures_local_name() {
    let info = parse_source("const mod = import('./service');");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./service");
    assert_eq!(info.dynamic_imports[0].local_name, Some("mod".to_string()));
}

#[test]
fn dynamic_import_destructured_captures_names() {
    let info =
        parse_source("async function f() { const { foo, bar } = await import('./module'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./module");
    assert!(info.dynamic_imports[0].local_name.is_none());
    assert_eq!(
        info.dynamic_imports[0].destructured_names,
        vec!["foo", "bar"]
    );
}

#[test]
fn dynamic_import_destructured_with_rest_is_namespace() {
    let info =
        parse_source("async function f() { const { foo, ...rest } = await import('./module'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./module");
    assert!(info.dynamic_imports[0].local_name.is_none());
    assert!(info.dynamic_imports[0].destructured_names.is_empty());
}

#[test]
fn dynamic_import_side_effect_only() {
    let info = parse_source("async function f() { await import('./side-effect'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./side-effect");
    assert!(info.dynamic_imports[0].local_name.is_none());
    assert!(info.dynamic_imports[0].destructured_names.is_empty());
}

#[test]
fn dynamic_import_no_duplicate_entries() {
    let info = parse_source("async function f() { const mod = await import('./service'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
}

// -- Namespace destructuring detection --

#[test]
fn namespace_destructuring_generates_member_accesses() {
    let info = parse_source("import * as utils from './utils';\nconst { foo, bar } = utils;");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.imports[0].imported_name, ImportedName::Namespace);
    let has_foo = info
        .member_accesses
        .iter()
        .any(|a| a.object == "utils" && a.member == "foo");
    let has_bar = info
        .member_accesses
        .iter()
        .any(|a| a.object == "utils" && a.member == "bar");
    assert!(
        has_foo,
        "Should capture destructured 'foo' as member access"
    );
    assert!(
        has_bar,
        "Should capture destructured 'bar' as member access"
    );
}

#[test]
fn namespace_destructuring_with_rest_marks_whole_object() {
    let info = parse_source("import * as utils from './utils';\nconst { foo, ...rest } = utils;");
    assert!(
        info.whole_object_uses.contains(&"utils".to_string()),
        "Rest pattern should mark namespace as whole-object use"
    );
}

#[test]
fn namespace_destructuring_from_dynamic_import() {
    let info = parse_source(
        "async function f() {\n  const mod = await import('./mod');\n  const { a, b } = mod;\n}",
    );
    let has_a = info
        .member_accesses
        .iter()
        .any(|a| a.object == "mod" && a.member == "a");
    let has_b = info
        .member_accesses
        .iter()
        .any(|a| a.object == "mod" && a.member == "b");
    assert!(
        has_a,
        "Should capture destructured 'a' from dynamic import namespace"
    );
    assert!(
        has_b,
        "Should capture destructured 'b' from dynamic import namespace"
    );
}

#[test]
fn namespace_destructuring_from_require() {
    let info = parse_source("const mod = require('./mod');\nconst { x, y } = mod;");
    let has_x = info
        .member_accesses
        .iter()
        .any(|a| a.object == "mod" && a.member == "x");
    let has_y = info
        .member_accesses
        .iter()
        .any(|a| a.object == "mod" && a.member == "y");
    assert!(
        has_x,
        "Should capture destructured 'x' from require namespace"
    );
    assert!(
        has_y,
        "Should capture destructured 'y' from require namespace"
    );
}

#[test]
fn non_namespace_destructuring_not_captured() {
    let info =
        parse_source("import { foo } from './utils';\nconst obj = { a: 1 };\nconst { a } = obj;");
    // 'obj' is not a namespace import, so destructuring should not add member_accesses for it
    let has_obj_a = info
        .member_accesses
        .iter()
        .any(|a| a.object == "obj" && a.member == "a");
    assert!(
        !has_obj_a,
        "Should not capture destructuring of non-namespace variables"
    );
}

// -- Unused import binding detection (oxc_semantic) --

#[test]
fn unused_import_binding_detected() {
    let info = parse_source("import { foo } from './utils';");
    assert!(
        info.unused_import_bindings.contains(&"foo".to_string()),
        "Import 'foo' is never used and should be in unused_import_bindings"
    );
}

#[test]
fn used_import_binding_not_in_unused() {
    let info = parse_source("import { foo } from './utils';\nconsole.log(foo);");
    assert!(
        !info.unused_import_bindings.contains(&"foo".to_string()),
        "Import 'foo' is used and should NOT be in unused_import_bindings"
    );
}

#[test]
fn unused_namespace_import_detected() {
    let info = parse_source("import * as utils from './utils';");
    assert!(
        info.unused_import_bindings.contains(&"utils".to_string()),
        "Namespace import 'utils' is never used and should be in unused_import_bindings"
    );
}

#[test]
fn used_namespace_import_not_in_unused() {
    let info = parse_source("import * as utils from './utils';\nutils.foo();");
    assert!(
        !info.unused_import_bindings.contains(&"utils".to_string()),
        "Namespace import 'utils' is used and should NOT be in unused_import_bindings"
    );
}

#[test]
fn reexported_import_not_in_unused() {
    let info = parse_source("import { foo } from './utils';\nexport { foo };");
    assert!(
        !info.unused_import_bindings.contains(&"foo".to_string()),
        "Import 'foo' is re-exported and should NOT be in unused_import_bindings"
    );
}

#[test]
fn type_only_import_used_as_type_not_in_unused() {
    let info = parse_source("import type { Foo } from './types';\nconst x: Foo = {} as any;");
    assert!(
        !info.unused_import_bindings.contains(&"Foo".to_string()),
        "Type import 'Foo' is used as a type annotation and should NOT be in unused_import_bindings"
    );
}

#[test]
fn value_import_used_only_as_type_not_in_unused() {
    // A value import (not `import type`) used only in a type annotation position
    // should NOT be in unused_import_bindings — oxc_semantic counts type-position
    // references as real references, which is correct since `import { Foo }` (without
    // the `type` keyword) may be needed at runtime depending on transpiler settings.
    let info = parse_source("import { Foo } from './types';\nconst x: Foo = {} as any;");
    assert!(
        !info.unused_import_bindings.contains(&"Foo".to_string()),
        "Value import 'Foo' used as type annotation should NOT be in unused_import_bindings"
    );
}

#[test]
fn side_effect_import_not_in_unused() {
    let info = parse_source("import './side-effect';");
    assert!(
        info.unused_import_bindings.is_empty(),
        "Side-effect imports have no binding and should not appear in unused_import_bindings"
    );
}

#[test]
fn mixed_used_and_unused_imports() {
    let info = parse_source("import { used, unused } from './utils';\nconsole.log(used);");
    assert!(
        !info.unused_import_bindings.contains(&"used".to_string()),
        "'used' is referenced"
    );
    assert!(
        info.unused_import_bindings.contains(&"unused".to_string()),
        "'unused' is not referenced"
    );
}

// -- Function overload deduplication --

#[test]
fn function_overloads_deduplicated_to_single_export() {
    let info = parse_source(
        "export function parse(): void;\nexport function parse(input: string): void;\nexport function parse(input?: string): void {}",
    );
    assert_eq!(
        info.exports.len(),
        1,
        "Function overloads should produce exactly 1 export, got {}",
        info.exports.len()
    );
    assert_eq!(info.exports[0].name, ExportName::Named("parse".to_string()));
}

// ---- JSDoc @public tag extraction tests ----

#[test]
fn jsdoc_public_tag_on_named_export() {
    let info = parse_source("/** @public */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_on_function_export() {
    let info = parse_source("/** @public */\nexport function bar() {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_on_default_export() {
    let info = parse_source("/** @public */\nexport default function main() {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_on_class_export() {
    let info = parse_source("/** @public */\nexport class Foo {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_on_type_export() {
    let info = parse_source("/** @public */\nexport type Foo = string;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_on_interface_export() {
    let info = parse_source("/** @public */\nexport interface Bar {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_on_enum_export() {
    let info = parse_source("/** @public */\nexport enum Status { Active, Inactive }");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_multiline() {
    let info = parse_source("/**\n * Some description.\n * @public\n */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_tag_with_other_tags() {
    let info = parse_source("/** @deprecated @public */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_api_public_tag() {
    let info = parse_source("/** @api public */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn no_jsdoc_tag_not_public() {
    let info = parse_source("export const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn line_comment_not_jsdoc() {
    // Only /** */ JSDoc comments count, not // comments
    let info = parse_source("// @public\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_does_not_match_public_foo() {
    // @publicFoo should NOT match @public
    let info = parse_source("/** @publicFoo */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_does_not_match_public_underscore() {
    // @public_api should NOT match @public (underscore is an identifier char)
    let info = parse_source("/** @public_api */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_apipublic_no_space_does_not_match() {
    // @apipublic (no space) should NOT match @api public
    let info = parse_source("/** @apipublic */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_export_specifier_list() {
    let source = "const foo = 1;\nconst bar = 2;\n/** @public */\nexport { foo, bar };";
    let info = parse_source(source);
    // @public on the export statement applies to all specifiers
    assert_eq!(info.exports.len(), 2);
    assert!(info.exports[0].is_public);
    assert!(info.exports[1].is_public);
}

#[test]
fn jsdoc_public_only_applies_to_attached_export() {
    let source = "/** @public */\nexport const foo = 1;\nexport const bar = 2;";
    let info = parse_source(source);
    assert_eq!(info.exports.len(), 2);
    assert!(info.exports[0].is_public);
    assert!(!info.exports[1].is_public);
}

// -- extract_destructured_names (tested indirectly) --

#[test]
fn require_destructured_empty_object() {
    let info = parse_source("const {} = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert!(info.require_calls[0].destructured_names.is_empty());
    assert!(info.require_calls[0].local_name.is_none());
}

#[test]
fn require_destructured_multiple_properties() {
    let info = parse_source("const { a, b, c } = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(
        info.require_calls[0].destructured_names,
        vec!["a", "b", "c"]
    );
}

#[test]
fn require_destructured_with_rest_returns_empty() {
    let info = parse_source("const { a, ...rest } = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert!(
        info.require_calls[0].destructured_names.is_empty(),
        "Rest element should cause extract_destructured_names to return empty vec"
    );
}

#[test]
fn require_destructured_computed_property_skipped() {
    // Computed property keys have no static name, so they are filtered out
    let info = parse_source("const key = 'x';\nconst { [key]: val, b } = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(
        info.require_calls[0].destructured_names,
        vec!["b"],
        "Computed property should be skipped, only 'b' captured"
    );
}

#[test]
fn require_destructured_aliased_properties() {
    // `{ foo: localFoo }` — the key name "foo" is what gets extracted
    let info = parse_source("const { foo: localFoo, bar: localBar } = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(
        info.require_calls[0].destructured_names,
        vec!["foo", "bar"],
        "Aliased destructured names should use the key (imported) name, not the local alias"
    );
}

#[test]
fn dynamic_import_destructured_empty_object() {
    let info = parse_source("async function f() { const {} = await import('./mod'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert!(info.dynamic_imports[0].destructured_names.is_empty());
    assert!(info.dynamic_imports[0].local_name.is_none());
}

#[test]
fn dynamic_import_destructured_computed_property_skipped() {
    let info =
        parse_source("async function f() { const { [key]: val, b } = await import('./mod'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(
        info.dynamic_imports[0].destructured_names,
        vec!["b"],
        "Computed property should be skipped in dynamic import destructuring"
    );
}

#[test]
fn dynamic_import_destructured_aliased_properties() {
    let info =
        parse_source("async function f() { const { foo: f1, bar: b1 } = await import('./mod'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(
        info.dynamic_imports[0].destructured_names,
        vec!["foo", "bar"],
        "Aliased destructured names should use the key name"
    );
}

// -- try_extract_require (tested indirectly) --

#[test]
fn require_with_variable_arg_not_captured() {
    let info = parse_source("const x = require(someVariable);");
    assert!(
        info.require_calls.is_empty(),
        "require() with a variable argument should not be captured"
    );
}

#[test]
fn require_with_template_literal_arg_not_captured() {
    let info = parse_source("const x = require(`./module`);");
    assert!(
        info.require_calls.is_empty(),
        "require() with a template literal should not be captured as a static require"
    );
}

#[test]
fn nested_require_inside_function_not_captured_as_declarator() {
    // `doSomething(require('foo'))` — this is NOT a `const x = require(...)` pattern,
    // but the visitor may still capture it as a bare require call
    let info = parse_source("doSomething(require('foo'));");
    // The bare require call is handled by visit_call_expression, not try_extract_require.
    // We verify the require is still detected through the general path.
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(info.require_calls[0].source, "foo");
    assert!(info.require_calls[0].local_name.is_none());
    assert!(info.require_calls[0].destructured_names.is_empty());
}

#[test]
fn require_with_non_require_callee_not_captured() {
    // A function called `notRequire` should not be treated as a require
    let info = parse_source("const x = notRequire('foo');");
    assert!(
        info.require_calls.is_empty(),
        "Only functions named 'require' should be captured"
    );
}

// -- try_extract_dynamic_import (tested indirectly) --

#[test]
fn dynamic_import_await_with_static_source() {
    let info = parse_source("async function f() { const m = await import('./svc'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./svc");
    assert_eq!(info.dynamic_imports[0].local_name, Some("m".to_string()));
}

#[test]
fn dynamic_import_without_await_with_static_source() {
    let info = parse_source("const p = import('./lazy');");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./lazy");
    assert_eq!(info.dynamic_imports[0].local_name, Some("p".to_string()));
}

#[test]
fn dynamic_import_with_template_literal_no_substitution() {
    // Template literal without expressions is treated as exact static import
    let info = parse_source("const m = import(`./exact`);");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./exact");
}

#[test]
fn dynamic_import_with_template_literal_expression_not_static() {
    // Template literal with expression — try_extract_dynamic_import returns None
    // (the source is not a StringLiteral), but the visitor handles it as a pattern
    let info = parse_source("const m = import(`./locales/${lang}`);");
    // Not captured as a static dynamic import
    assert!(
        !info.dynamic_imports.iter().any(|d| d.source.contains("${")),
        "Template literal with expression should not appear as static dynamic import source"
    );
    // But captured as a dynamic import pattern
    assert_eq!(info.dynamic_import_patterns.len(), 1);
}

#[test]
fn await_non_import_expression_not_captured() {
    // `await someFunc()` should not be treated as a dynamic import
    let info = parse_source("async function f() { const x = await someFunc(); }");
    assert!(
        info.dynamic_imports.is_empty(),
        "await of a non-import expression should not be captured as dynamic import"
    );
}

// ── JSX retry fallback ──────────────────────────────────────────

/// Parse as a .js file (not .tsx) to test JSX retry fallback logic.
fn parse_as_js(source: &str) -> ModuleInfo {
    parse_source_to_module(FileId(0), Path::new("component.js"), source, 0)
}

#[test]
fn jsx_retry_extracts_exports_from_js_with_jsx() {
    // A .js file with JSX that the initial non-JSX parse can't extract anything from.
    // Must be >100 bytes and have total_extracted == 0 on first pass to trigger retry.
    // The initial parse of .js without JSX mode will fail on JSX tags and extract nothing.
    let source = r#"
export const App = () => <div className="app"><span>Hello World from JSX in a plain JS file</span></div>;
"#;
    let info = parse_as_js(source);
    assert!(
        !info.exports.is_empty(),
        "JSX retry should extract the App export from .js file with JSX"
    );
}

#[test]
fn jsx_retry_extracts_imports_from_js_with_jsx() {
    // File with both import and JSX — the initial .js parse may still extract the import
    // (imports before JSX tags often parse fine), so this tests robustness.
    let source = r#"
export default function Component() {
    return <main><section className="hero"><h1>Title</h1><p>Description paragraph</p></section></main>;
}
"#;
    let info = parse_as_js(source);
    assert!(
        !info.exports.is_empty(),
        "JSX retry should extract the default export from .js file with JSX"
    );
}

#[test]
fn jsx_retry_preserves_jsdoc_public_tag() {
    // Regression: @public tags were read from the original failed parse's comments
    // instead of the retry parse's comments, silently ignoring @public on JSX .js files.
    let source = r#"
/** @public */
export const Button = ({ children }) => <button className="btn">{children}</button>;
"#;
    let info = parse_as_js(source);
    assert!(
        !info.exports.is_empty(),
        "JSX retry should extract Button export"
    );
    assert!(
        info.exports[0].is_public,
        "@public JSDoc tag must be recognized on JSX exports in .js files"
    );
}

#[test]
fn jsx_retry_preserves_suppressions() {
    // Regression: suppression comments were parsed from the original failed parse's
    // comments instead of the retry parse's comments.
    let source = r#"
// fallow-ignore-next-line unused-export
export const Unused = ({ text }) => <span className="unused-component">{text}</span>;
"#;
    let info = parse_as_js(source);
    assert!(
        !info.suppressions.is_empty(),
        "Suppressions must be parsed from retry parse comments, not the original failed parse"
    );
}

// ---- Additional JSDoc @public tag tests ----

#[test]
fn jsdoc_public_block_comment_not_jsdoc() {
    // /* @public */ is a block comment, not a JSDoc comment (requires /**)
    let info = parse_source("/* @public */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_anonymous_default_export() {
    let info = parse_source("/** @public */\nexport default function() {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_arrow_default_export() {
    let info = parse_source("/** @public */\nexport default () => {};");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_default_expression_export() {
    let info = parse_source("/** @public */\nexport default 42;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_let_export() {
    let info = parse_source("/** @public */\nexport let count = 0;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("count".to_string()));
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_with_trailing_description() {
    // @public followed by descriptive text (space-separated) should still match
    let info = parse_source("/** @public This is always exported */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_api_public_with_extra_whitespace() {
    // @api followed by multiple spaces then public
    let info = parse_source("/** @api   public */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_api_public_with_newline() {
    // @api on one line, public on the next
    let info = parse_source("/**\n * @api\n * public\n */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    // trim_start includes newlines, so "* public\n */" starts with "* public", not "public"
    // This should NOT match because there is a "* " prefix before "public"
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_api_publicfoo_does_not_match() {
    // @api publicFoo should not match (publicFoo is not standalone "public")
    let info = parse_source("/** @api publicFoo */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_multiple_exports_all_tagged() {
    let source = "/** @public */\nexport const a = 1;\n/** @public */\nexport const b = 2;";
    let info = parse_source(source);
    assert_eq!(info.exports.len(), 2);
    assert!(info.exports[0].is_public);
    assert!(info.exports[1].is_public);
}

#[test]
fn jsdoc_public_mixed_three_exports() {
    let source = "/** @public */\nexport const a = 1;\nexport const b = 2;\n/** @public */\nexport const c = 3;";
    let info = parse_source(source);
    assert_eq!(info.exports.len(), 3);
    assert!(info.exports[0].is_public);
    assert!(!info.exports[1].is_public);
    assert!(info.exports[2].is_public);
}

#[test]
fn jsdoc_public_does_not_match_numeric_suffix() {
    // @public2 should NOT match @public (digit is an ident char)
    let info = parse_source("/** @public2 */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_async_function_export() {
    let info = parse_source("/** @public */\nexport async function fetchData() {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_abstract_class_export() {
    let info = parse_source("/** @public */\nexport abstract class Base {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_star_prefix_in_multiline() {
    // Standard JSDoc with * prefix on each line
    let info = parse_source(
        "/**\n * @param x - the value\n * @returns the result\n * @public\n */\nexport const foo = 1;",
    );
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_public_on_type_alias_union() {
    let info = parse_source("/** @public */\nexport type Status = 'active' | 'inactive';");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_api_public_on_function() {
    let info = parse_source("/** @api public */\nexport function handler() {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_public);
}

#[test]
fn jsdoc_api_private_does_not_set_public() {
    // @api private is not @api public
    let info = parse_source("/** @api private */\nexport const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

#[test]
fn jsdoc_public_not_leaked_across_statements() {
    // The @public tag is on a non-export statement; the export that follows should NOT inherit it
    let source = "/** @public */\nconst internal = 1;\nexport const foo = internal;";
    let info = parse_source(source);
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_public);
}

// ── Cyclomatic & cognitive complexity (via ModuleInfo.complexity) ──

#[test]
fn complexity_basic_if_else_for_while_switch() {
    let info = parse_source(
        r"function basic(x: number) {
            if (x > 10) {
                return 'big';
            } else {
                for (let i = 0; i < x; i++) {}
                while (x > 0) { x--; }
                switch (x) {
                    case 0: break;
                    case 1: break;
                    default: break;
                }
            }
        }",
    );
    let f = info.complexity.iter().find(|c| c.name == "basic").unwrap();
    // 1 (base) + if + for + while + case + case = 6 (default: not counted)
    assert_eq!(f.cyclomatic, 6);
}

#[test]
fn complexity_nested_if_in_for_loop() {
    let info = parse_source(
        r"function nested(items: number[]) {
            for (const item of items) {
                if (item > 0) {
                    return item;
                }
            }
        }",
    );
    let f = info.complexity.iter().find(|c| c.name == "nested").unwrap();
    // Cyclomatic: 1 + for_of + if = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: for_of +1 (n=0), if +1+1 (n=1) = 3
    assert_eq!(f.cognitive, 3);
}

#[test]
fn complexity_deeply_nested_three_levels() {
    let info = parse_source(
        r"function deep(a: boolean, b: boolean, c: boolean) {
            if (a) {
                for (let i = 0; i < 10; i++) {
                    while (b) {
                        if (c) {
                            break;
                        }
                    }
                }
            }
        }",
    );
    let f = info.complexity.iter().find(|c| c.name == "deep").unwrap();
    // Cyclomatic: 1 + if + for + while + if = 5
    assert_eq!(f.cyclomatic, 5);
    // Cognitive: if +1 (n=0), for +1+1 (n=1), while +1+2 (n=2), if +1+3 (n=3) = 1+2+3+4 = 10
    assert_eq!(f.cognitive, 10);
}

#[test]
fn complexity_boolean_same_operator_sequence() {
    let info = parse_source(
        "function sameBool(a: boolean, b: boolean, c: boolean) { return a && b && c; }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "sameBool")
        .unwrap();
    // Cyclomatic: 1 + && + && = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: same operator throughout = +1
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_boolean_mixed_operator_sequence() {
    let info = parse_source(
        "function mixedBool(a: boolean, b: boolean, c: boolean) { return a && b || c; }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "mixedBool")
        .unwrap();
    // Cyclomatic: 1 + && + || = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: && starts sequence +1, || changes operator +1 = 2
    assert_eq!(f.cognitive, 2);
}

#[test]
fn complexity_boolean_three_operator_changes() {
    let info = parse_source(
        "function threeBool(a: boolean, b: boolean, c: boolean, d: boolean) { return a && b || c && d; }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "threeBool")
        .unwrap();
    // Cyclomatic: 1 + && + || + && = 4
    assert_eq!(f.cyclomatic, 4);
    // Cognitive: && +1, || +1, && +1 = 3
    assert_eq!(f.cognitive, 3);
}

#[test]
fn complexity_ternary_operator() {
    let info = parse_source("function tern(x: number) { return x > 0 ? 'pos' : 'non-pos'; }");
    let f = info.complexity.iter().find(|c| c.name == "tern").unwrap();
    // Cyclomatic: 1 + ternary = 2
    assert_eq!(f.cyclomatic, 2);
    // Cognitive: ternary +1 (n=0) = 1
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_nested_ternary() {
    let info = parse_source(
        "function nestedTern(x: number) { return x > 0 ? 'pos' : x < 0 ? 'neg' : 'zero'; }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "nestedTern")
        .unwrap();
    // Cyclomatic: 1 + ternary + ternary = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: outer ternary +1 (n=0), inner ternary +1+1 (n=1) = 3
    assert_eq!(f.cognitive, 3);
}

#[test]
fn complexity_try_catch() {
    let info = parse_source(
        r"function tryCatch() {
            try {
                riskyOp();
            } catch (e) {
                handleError(e);
            }
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "tryCatch")
        .unwrap();
    // Cyclomatic: 1 + catch = 2
    assert_eq!(f.cyclomatic, 2);
    // Cognitive: catch +1 (n=0) = 1
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_try_catch_with_nested_if() {
    let info = parse_source(
        r"function tryCatchNested(x: boolean) {
            try {
                if (x) { riskyOp(); }
            } catch (e) {
                if (e instanceof Error) { log(e); }
            }
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "tryCatchNested")
        .unwrap();
    // Cyclomatic: 1 + if + catch + if = 4
    assert_eq!(f.cyclomatic, 4);
    // Cognitive: if +1 (n=0), catch +1 (n=0), if inside catch +1+1 (n=1) = 4
    assert_eq!(f.cognitive, 4);
}

#[test]
fn complexity_nested_functions_independent() {
    let info = parse_source(
        r"function outer(x: boolean) {
            if (x) {}
            function inner(y: boolean) {
                if (y) {
                    if (y) {}
                }
            }
        }",
    );
    let outer = info.complexity.iter().find(|c| c.name == "outer").unwrap();
    let inner = info.complexity.iter().find(|c| c.name == "inner").unwrap();
    // outer: 1 + if = 2 cyclomatic, if +1 = 1 cognitive
    assert_eq!(outer.cyclomatic, 2);
    assert_eq!(outer.cognitive, 1);
    // inner: 1 + if + if = 3 cyclomatic, if +1 (n=0) + if +1+1 (n=1) = 3 cognitive
    assert_eq!(inner.cyclomatic, 3);
    assert_eq!(inner.cognitive, 3);
}

#[test]
fn complexity_arrow_function_in_callback() {
    let info = parse_source(
        r"function process(items: number[]) {
            items.map((item) => {
                if (item > 0) {
                    return item * 2;
                }
                return 0;
            });
        }",
    );
    let outer = info
        .complexity
        .iter()
        .find(|c| c.name == "process")
        .unwrap();
    let arrow = info
        .complexity
        .iter()
        .find(|c| c.name == "<arrow>")
        .unwrap();
    // outer: base 1 only (no decisions in outer scope)
    assert_eq!(outer.cyclomatic, 1);
    assert_eq!(outer.cognitive, 0);
    // arrow: 1 + if = 2 cyclomatic, if +1 (n=0, reset for new function) = 1 cognitive
    assert_eq!(arrow.cyclomatic, 2);
    assert_eq!(arrow.cognitive, 1);
}

#[test]
fn complexity_named_arrow_in_variable() {
    let info = parse_source(
        r"function process(items: number[]) {
            const filter = (item: number) => item > 0;
            return items.filter(filter);
        }",
    );
    let arrow = info.complexity.iter().find(|c| c.name == "filter").unwrap();
    // Arrow with no decisions: base 1 cyclomatic, 0 cognitive
    assert_eq!(arrow.cyclomatic, 1);
    assert_eq!(arrow.cognitive, 0);
}

#[test]
fn complexity_class_methods_independent() {
    let info = parse_source(
        r"class Parser {
            parse(input: string) {
                if (input.length === 0) { return null; }
                for (let i = 0; i < input.length; i++) {
                    if (input[i] === '{') { return this.parseObject(input); }
                }
                return input;
            }
            validate(input: string) {
                return input ? true : false;
            }
        }",
    );
    let parse = info.complexity.iter().find(|c| c.name == "parse").unwrap();
    let validate = info
        .complexity
        .iter()
        .find(|c| c.name == "validate")
        .unwrap();
    // parse: 1 + if + for + if = 4
    assert_eq!(parse.cyclomatic, 4);
    // parse cognitive: if +1 (n=0), for +1 (n=0), if +1+1 (n=1) = 4
    assert_eq!(parse.cognitive, 4);
    // validate: 1 + ternary = 2
    assert_eq!(validate.cyclomatic, 2);
    // validate cognitive: ternary +1 (n=0) = 1
    assert_eq!(validate.cognitive, 1);
}

#[test]
fn complexity_class_property_arrow() {
    let info = parse_source(
        r"class Handler {
            handle = (x: number) => {
                if (x > 0) { return x; }
                return 0;
            };
        }",
    );
    let handle = info.complexity.iter().find(|c| c.name == "handle").unwrap();
    // 1 + if = 2
    assert_eq!(handle.cyclomatic, 2);
    assert_eq!(handle.cognitive, 1);
}

#[test]
fn complexity_nullish_coalescing() {
    let info = parse_source("function nc(a?: string) { return a ?? 'default'; }");
    let f = info.complexity.iter().find(|c| c.name == "nc").unwrap();
    // Cyclomatic: 1 + ?? = 2
    assert_eq!(f.cyclomatic, 2);
    // Cognitive: ?? is a logical operator, gets +1 for the sequence
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_nullish_coalescing_chain() {
    let info =
        parse_source("function ncChain(a?: string, b?: string) { return a ?? b ?? 'default'; }");
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "ncChain")
        .unwrap();
    // Cyclomatic: 1 + ?? + ?? = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: same operator ?? throughout = +1
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_logical_and_assignment() {
    let info = parse_source("function la(obj: any) { obj.value &&= 'assigned'; }");
    let f = info.complexity.iter().find(|c| c.name == "la").unwrap();
    // Cyclomatic: 1 + &&= = 2
    assert_eq!(f.cyclomatic, 2);
}

#[test]
fn complexity_logical_or_assignment() {
    let info = parse_source("function lo(obj: any) { obj.value ||= 'fallback'; }");
    let f = info.complexity.iter().find(|c| c.name == "lo").unwrap();
    // Cyclomatic: 1 + ||= = 2
    assert_eq!(f.cyclomatic, 2);
}

#[test]
fn complexity_nullish_assignment() {
    let info = parse_source("function na(obj: any) { obj.value ??= 'default'; }");
    let f = info.complexity.iter().find(|c| c.name == "na").unwrap();
    // Cyclomatic: 1 + ??= = 2
    assert_eq!(f.cyclomatic, 2);
}

#[test]
fn complexity_all_logical_assignments() {
    let info = parse_source("function allAssign(o: any) { o.a &&= 1; o.b ||= 2; o.c ??= 3; }");
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "allAssign")
        .unwrap();
    // Cyclomatic: 1 + &&= + ||= + ??= = 4
    assert_eq!(f.cyclomatic, 4);
}

#[test]
fn complexity_optional_chaining_cyclomatic_only() {
    let info = parse_source("function oc(obj: any) { return obj?.a?.b; }");
    let f = info.complexity.iter().find(|c| c.name == "oc").unwrap();
    // Cyclomatic: optional chaining adds to cyclomatic
    assert!(
        f.cyclomatic >= 2,
        "optional chaining should add to cyclomatic"
    );
    // Cognitive: optional chaining is NOT counted (Principle 3)
    assert_eq!(f.cognitive, 0);
}

#[test]
fn complexity_do_while_loop() {
    let info = parse_source(
        r"function doWhile(x: number) {
            do {
                x--;
            } while (x > 0);
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "doWhile")
        .unwrap();
    // Cyclomatic: 1 + do-while = 2
    assert_eq!(f.cyclomatic, 2);
    // Cognitive: do-while +1 (n=0) = 1
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_for_in_loop() {
    let info = parse_source(
        r"function forIn(obj: Record<string, number>) {
            for (const key in obj) {
                if (obj[key] > 0) {}
            }
        }",
    );
    let f = info.complexity.iter().find(|c| c.name == "forIn").unwrap();
    // Cyclomatic: 1 + for-in + if = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: for-in +1 (n=0), if +1+1 (n=1) = 3
    assert_eq!(f.cognitive, 3);
}

#[test]
fn complexity_switch_cognitive_is_flat() {
    let info = parse_source(
        r"function sw(x: number) {
            switch (x) {
                case 1: return 'one';
                case 2: return 'two';
                case 3: return 'three';
                default: return 'other';
            }
        }",
    );
    let f = info.complexity.iter().find(|c| c.name == "sw").unwrap();
    // Cyclomatic: 1 + case + case + case = 4 (default: not counted)
    assert_eq!(f.cyclomatic, 4);
    // Cognitive: switch +1 (not per-case)
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_else_if_chain_cognitive_flat() {
    let info = parse_source(
        r"function elseIfChain(x: number) {
            if (x === 1) {
                return 'one';
            } else if (x === 2) {
                return 'two';
            } else if (x === 3) {
                return 'three';
            } else {
                return 'other';
            }
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "elseIfChain")
        .unwrap();
    // Cyclomatic: 1 + if + else-if + else-if = 4
    assert_eq!(f.cyclomatic, 4);
    // Cognitive: if +1, else if +1 (flat), else if +1 (flat), else +1 (flat) = 4
    assert_eq!(f.cognitive, 4);
}

#[test]
fn complexity_break_with_label() {
    let info = parse_source(
        r"function labeled() {
            outer: for (let i = 0; i < 10; i++) {
                for (let j = 0; j < 10; j++) {
                    if (i + j > 5) {
                        break outer;
                    }
                }
            }
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "labeled")
        .unwrap();
    // Cyclomatic: 1 + for + for + if = 4
    assert_eq!(f.cyclomatic, 4);
    // Cognitive: for +1 (n=0), for +1+1 (n=1), if +1+2 (n=2), break label +1 (flat) = 7
    assert_eq!(f.cognitive, 7);
}

#[test]
fn complexity_continue_with_label() {
    let info = parse_source(
        r"function labeledContinue() {
            outer: for (let i = 0; i < 10; i++) {
                for (let j = 0; j < 10; j++) {
                    if (j === 3) {
                        continue outer;
                    }
                }
            }
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "labeledContinue")
        .unwrap();
    // Cognitive includes +1 for continue label
    assert_eq!(f.cognitive, 7);
}

#[test]
fn complexity_mixed_boolean_with_nullish() {
    let info = parse_source(
        "function mixedNullish(a: boolean, b?: string) { return a && b ?? 'default'; }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "mixedNullish")
        .unwrap();
    // Cyclomatic: 1 + && + ?? = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: && starts +1, ?? changes operator +1 = 2
    assert_eq!(f.cognitive, 2);
}

#[test]
fn complexity_boolean_in_if_condition() {
    let info = parse_source(
        r"function boolInIf(a: boolean, b: boolean) {
            if (a && b) {
                return true;
            }
            return false;
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "boolInIf")
        .unwrap();
    // Cyclomatic: 1 + if + && = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: if +1 (n=0) + && +1 (flat, boolean sequence) = 2
    assert_eq!(f.cognitive, 2);
}

#[test]
fn complexity_multiple_independent_functions() {
    let info = parse_source(
        r"
        function a(x: boolean) { if (x) {} }
        function b(x: boolean, y: boolean) { if (x) { if (y) {} } }
        function c() {}
        ",
    );
    let fa = info.complexity.iter().find(|c| c.name == "a").unwrap();
    let fb = info.complexity.iter().find(|c| c.name == "b").unwrap();
    let fc = info.complexity.iter().find(|c| c.name == "c").unwrap();
    assert_eq!(fa.cyclomatic, 2);
    assert_eq!(fa.cognitive, 1);
    assert_eq!(fb.cyclomatic, 3);
    assert_eq!(fb.cognitive, 3);
    assert_eq!(fc.cyclomatic, 1);
    assert_eq!(fc.cognitive, 0);
}

#[test]
fn complexity_export_default_anonymous_function() {
    let info = parse_source("export default function() { if (true) { while (true) {} } }");
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "default")
        .unwrap();
    // Cyclomatic: 1 + if + while = 3
    assert_eq!(f.cyclomatic, 3);
    // Cognitive: if +1 (n=0), while +1+1 (n=1) = 3
    assert_eq!(f.cognitive, 3);
}

#[test]
fn complexity_object_method_shorthand() {
    let info = parse_source(
        r"const obj = {
            process(x: number) {
                if (x > 0) { return x; }
                return 0;
            }
        };",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "process")
        .unwrap();
    assert_eq!(f.cyclomatic, 2);
    assert_eq!(f.cognitive, 1);
}

#[test]
fn complexity_catch_increases_nesting() {
    let info = parse_source(
        r"function tryCatchDeep() {
            try {
                riskyOp();
            } catch (e) {
                if (e instanceof Error) {
                    for (const c of e.message) {
                        log(c);
                    }
                }
            }
        }",
    );
    let f = info
        .complexity
        .iter()
        .find(|c| c.name == "tryCatchDeep")
        .unwrap();
    // Cyclomatic: 1 + catch + if + for_of = 4
    assert_eq!(f.cyclomatic, 4);
    // Cognitive: catch +1 (n=0), if +1+1 (n=1), for_of +1+2 (n=2) = 6
    assert_eq!(f.cognitive, 6);
}

// ── Declaration extraction edge cases ────────────────────────────

#[test]
fn enum_with_string_values_extracts_members() {
    let info = parse_source(
        "export enum Status { Active = \"ACTIVE\", Inactive = \"INACTIVE\", Pending = \"PENDING\" }",
    );
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("Status".to_string())
    );
    assert_eq!(info.exports[0].members.len(), 3);
    let names: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(names, vec!["Active", "Inactive", "Pending"]);
    assert!(
        info.exports[0]
            .members
            .iter()
            .all(|m| m.kind == MemberKind::EnumMember)
    );
}

#[test]
fn enum_with_numeric_values_extracts_members() {
    let info = parse_source("export enum HttpCode { OK = 200, NotFound = 404, ServerError = 500 }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 3);
    let names: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(names, vec!["OK", "NotFound", "ServerError"]);
}

#[test]
fn enum_not_type_only() {
    // Enums are runtime values, not type-only
    let info = parse_source("export enum Color { Red, Green, Blue }");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_type_only);
}

#[test]
fn const_enum_not_type_only() {
    let info = parse_source("export const enum Direction { Up, Down }");
    assert_eq!(info.exports.len(), 1);
    // const enums are still exported as values (unless isolated modules)
    assert!(!info.exports[0].is_type_only);
}

#[test]
fn abstract_class_export_single_export() {
    let info = parse_source("export abstract class Base { abstract doWork(): void; }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("Base".to_string()));
    assert!(!info.exports[0].is_type_only);
}

#[test]
fn abstract_class_with_concrete_members() {
    let info = parse_source(
        r"export abstract class Base {
            abstract doWork(): void;
            getName() { return 'base'; }
            label: string = 'base';
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let members: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    // Abstract methods and concrete methods/properties are all tracked
    assert!(members.contains(&"doWork"));
    assert!(members.contains(&"getName"));
    assert!(members.contains(&"label"));
}

#[test]
fn class_private_members_excluded() {
    let info = parse_source(
        r"export class Svc {
            private secret: string = '';
            private doSecret() {}
            public visible() {}
            name: string = '';
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let names: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert!(
        !names.contains(&"secret"),
        "Private property should be excluded"
    );
    assert!(
        !names.contains(&"doSecret"),
        "Private method should be excluded"
    );
    assert!(
        names.contains(&"visible"),
        "Public method should be included"
    );
    assert!(
        names.contains(&"name"),
        "Unadorned property should be included"
    );
}

#[test]
fn class_protected_members_excluded() {
    let info = parse_source(
        r"export class Base {
            protected internalMethod() {}
            protected internalProp: number = 0;
            publicMethod() {}
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let names: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert!(
        !names.contains(&"internalMethod"),
        "Protected method should be excluded"
    );
    assert!(
        !names.contains(&"internalProp"),
        "Protected property should be excluded"
    );
    assert!(
        names.contains(&"publicMethod"),
        "Public method should be included"
    );
}

#[test]
fn class_decorated_members_tracked() {
    let info = parse_source(
        r"export class Controller {
            @Get('/users')
            getUsers() { return []; }
            @Post('/users')
            createUser() {}
            plain() {}
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let get_users = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "getUsers")
        .expect("getUsers should be in members");
    assert!(
        get_users.has_decorator,
        "getUsers should have has_decorator = true"
    );
    let create_user = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "createUser")
        .expect("createUser should be in members");
    assert!(
        create_user.has_decorator,
        "createUser should have has_decorator = true"
    );
    let plain = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "plain")
        .expect("plain should be in members");
    assert!(
        !plain.has_decorator,
        "plain should have has_decorator = false"
    );
}

#[test]
fn class_decorated_properties_tracked() {
    let info = parse_source(
        r"export class Entity {
            @Column()
            name: string = '';
            @Column()
            age: number = 0;
            undecorated: boolean = false;
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let name_prop = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "name")
        .expect("name should be in members");
    assert!(name_prop.has_decorator);
    assert_eq!(name_prop.kind, MemberKind::ClassProperty);
    let undecorated = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "undecorated")
        .expect("undecorated should be in members");
    assert!(!undecorated.has_decorator);
}

#[test]
fn class_member_kinds_correct() {
    let info = parse_source(
        r"export class MyClass {
            method() {}
            prop: string = '';
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let method = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "method")
        .unwrap();
    assert_eq!(method.kind, MemberKind::ClassMethod);
    let prop = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "prop")
        .unwrap();
    assert_eq!(prop.kind, MemberKind::ClassProperty);
}

#[test]
fn function_overloads_different_names_not_deduplicated() {
    let info = parse_source("export function foo(): void {}\nexport function bar(): void {}");
    assert_eq!(
        info.exports.len(),
        2,
        "Different function names should produce separate exports"
    );
    assert_eq!(info.exports[0].name, ExportName::Named("foo".to_string()));
    assert_eq!(info.exports[1].name, ExportName::Named("bar".to_string()));
}

#[test]
fn function_overloads_many_signatures_single_export() {
    let info = parse_source(
        r"export function create(): void;
export function create(name: string): void;
export function create(name: string, age: number): void;
export function create(name?: string, age?: number): void {}",
    );
    assert_eq!(
        info.exports.len(),
        1,
        "Four overload signatures should deduplicate to 1 export"
    );
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("create".to_string())
    );
}

#[test]
fn multiple_variable_declarations_in_one_export() {
    let info = parse_source("export const a = 1, b = 'two', c = true;");
    assert_eq!(info.exports.len(), 3);
    assert_eq!(info.exports[0].name, ExportName::Named("a".to_string()));
    assert_eq!(info.exports[1].name, ExportName::Named("b".to_string()));
    assert_eq!(info.exports[2].name, ExportName::Named("c".to_string()));
}

#[test]
fn destructured_export_with_defaults() {
    let info = parse_source("export const { a = 1, b = 2 } = obj;");
    assert_eq!(info.exports.len(), 2);
    assert_eq!(info.exports[0].name, ExportName::Named("a".to_string()));
    assert_eq!(info.exports[1].name, ExportName::Named("b".to_string()));
}

#[test]
fn deeply_nested_array_destructured_export() {
    let info = parse_source("export const [[a], [b, c]] = nested;");
    assert_eq!(info.exports.len(), 3);
    assert_eq!(info.exports[0].name, ExportName::Named("a".to_string()));
    assert_eq!(info.exports[1].name, ExportName::Named("b".to_string()));
    assert_eq!(info.exports[2].name, ExportName::Named("c".to_string()));
}

#[test]
fn mixed_object_array_destructured_export() {
    let info = parse_source("export const { items: [first, second] } = config;");
    assert_eq!(info.exports.len(), 2);
    assert_eq!(info.exports[0].name, ExportName::Named("first".to_string()));
    assert_eq!(
        info.exports[1].name,
        ExportName::Named("second".to_string())
    );
}

#[test]
fn destructured_export_with_rename() {
    let info = parse_source("export const { original: renamed } = obj;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("renamed".to_string())
    );
}

#[test]
fn require_namespace_binding_captures_local_name() {
    let info = parse_source("const fs = require('fs');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(info.require_calls[0].source, "fs");
    assert_eq!(
        info.require_calls[0].local_name,
        Some("fs".to_string()),
        "Namespace require should capture the local binding name"
    );
    assert!(info.require_calls[0].destructured_names.is_empty());
}

#[test]
fn require_destructured_no_local_name() {
    let info = parse_source("const { readFile, writeFile } = require('fs');");
    assert_eq!(info.require_calls.len(), 1);
    assert!(
        info.require_calls[0].local_name.is_none(),
        "Destructured require should have no local_name"
    );
    assert_eq!(
        info.require_calls[0].destructured_names,
        vec!["readFile", "writeFile"]
    );
}

#[test]
fn ts_module_declaration_identifier() {
    let info = parse_source("export declare module MyModule {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("MyModule".to_string())
    );
    assert!(info.exports[0].is_type_only);
}

#[test]
fn ts_namespace_declaration() {
    let info = parse_source("export namespace Utils { export function helper() {} }");
    // Namespace produces an export for the namespace name, plus inner exports
    assert!(
        info.exports
            .iter()
            .any(|e| e.name == ExportName::Named("Utils".to_string())),
        "Should contain the namespace export"
    );
    assert!(
        info.exports
            .iter()
            .find(|e| e.name == ExportName::Named("Utils".to_string()))
            .unwrap()
            .is_type_only
    );
}

#[test]
fn export_let_declaration() {
    let info = parse_source("export let mutable = 42;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("mutable".to_string())
    );
}

#[test]
fn export_var_declaration() {
    let info = parse_source("export var legacy = 'old';");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("legacy".to_string())
    );
}

#[test]
fn export_async_function() {
    let info = parse_source("export async function fetchData() {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("fetchData".to_string())
    );
    assert!(!info.exports[0].is_type_only);
}

#[test]
fn export_generator_function() {
    let info = parse_source("export function* generateItems() { yield 1; }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("generateItems".to_string())
    );
}

#[test]
fn type_alias_always_type_only() {
    let info = parse_source(
        "export type Result<T> = { ok: true; data: T } | { ok: false; error: string };",
    );
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_type_only);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("Result".to_string())
    );
}

#[test]
fn interface_always_type_only() {
    let info = parse_source(
        "export interface Config { debug: boolean; verbose: boolean; output: string; }",
    );
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_type_only);
}

#[test]
fn interface_extending_another_type_only() {
    let info =
        parse_source("export interface ExtendedConfig extends BaseConfig { extra: boolean; }");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_type_only);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("ExtendedConfig".to_string())
    );
}

#[test]
fn dynamic_import_then_destructuring_captures_member_accesses() {
    let info = parse_source(
        r"async function load() {
            const mod = await import('./service');
            const { handler, middleware } = mod;
        }",
    );
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].local_name, Some("mod".to_string()));
    let has_handler = info
        .member_accesses
        .iter()
        .any(|a| a.object == "mod" && a.member == "handler");
    let has_middleware = info
        .member_accesses
        .iter()
        .any(|a| a.object == "mod" && a.member == "middleware");
    assert!(
        has_handler,
        "Should capture 'handler' from namespace destructuring"
    );
    assert!(
        has_middleware,
        "Should capture 'middleware' from namespace destructuring"
    );
}

#[test]
fn namespace_destructuring_rest_marks_whole_object_for_require() {
    let info = parse_source("const mod = require('./mod');\nconst { a, ...rest } = mod;");
    assert!(
        info.whole_object_uses.contains(&"mod".to_string()),
        "Rest pattern in require namespace destructuring should mark whole-object use"
    );
}

#[test]
fn export_default_class() {
    let info = parse_source("export default class MyClass {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_default_anonymous_class() {
    let info = parse_source("export default class {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_default_expression_literal() {
    let info = parse_source("export default 'hello';");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_default_object_expression() {
    let info = parse_source("export default { key: 'value' };");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn class_static_method_tracked() {
    let info = parse_source(
        r"export class Factory {
            static create() { return new Factory(); }
            instance() {}
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let names: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert!(names.contains(&"create"), "Static method should be tracked");
    assert!(
        names.contains(&"instance"),
        "Instance method should be tracked"
    );
}

#[test]
fn class_getter_setter_tracked() {
    let info = parse_source(
        r"export class Config {
            get value() { return this._value; }
            set value(v: string) { this._value = v; }
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let has_value = info.exports[0].members.iter().any(|m| m.name == "value");
    assert!(has_value, "Getter/setter should be tracked as member");
}

#[test]
fn enum_single_member() {
    let info = parse_source("export enum Single { Only }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 1);
    assert_eq!(info.exports[0].members[0].name, "Only");
}

#[test]
fn enum_empty() {
    let info = parse_source("export enum Empty {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].members.is_empty());
}

#[test]
fn enum_string_literal_member_name() {
    // Enum members can use string literal keys
    let info = parse_source("export enum Weird { 'hello-world' = 1 }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 1);
    assert_eq!(info.exports[0].members[0].name, "hello-world");
}

#[test]
fn multiple_type_exports_all_type_only() {
    let info = parse_source(
        "export type A = string;\nexport type B = number;\nexport interface C { x: boolean; }",
    );
    assert_eq!(info.exports.len(), 3);
    assert!(info.exports.iter().all(|e| e.is_type_only));
}

#[test]
fn mixed_value_and_type_exports() {
    let info = parse_source(
        "export const value = 1;\nexport type TypeAlias = string;\nexport function fn() {}",
    );
    assert_eq!(info.exports.len(), 3);
    assert!(
        !info.exports[0].is_type_only,
        "const should not be type-only"
    );
    assert!(
        info.exports[1].is_type_only,
        "type alias should be type-only"
    );
    assert!(
        !info.exports[2].is_type_only,
        "function should not be type-only"
    );
}

#[test]
fn array_destructured_export_with_skip() {
    // Skipping elements in array destructuring with holes
    let info = parse_source("export const [, second, , fourth] = arr;");
    assert_eq!(info.exports.len(), 2);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("second".to_string())
    );
    assert_eq!(
        info.exports[1].name,
        ExportName::Named("fourth".to_string())
    );
}

#[test]
fn export_local_name_matches_for_simple_declarations() {
    let info = parse_source("export const foo = 1;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].local_name,
        Some("foo".to_string()),
        "local_name should match the binding name"
    );
}

#[test]
fn export_specifier_with_as_default() {
    // `export { foo as default }` uses a named specifier with "default" as the exported name
    let info = parse_source("const foo = 1;\nexport { foo as default };");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("default".to_string())
    );
}

// ── Class member extraction: static properties ──────────────

#[test]
fn class_static_property_tracked() {
    let info = parse_source(
        r"export class Foo {
            static count = 0;
            static label: string = 'default';
            regular: number = 1;
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let names: Vec<&str> = info.exports[0]
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert!(
        names.contains(&"count"),
        "Static property 'count' should be tracked"
    );
    assert!(
        names.contains(&"label"),
        "Static property 'label' should be tracked"
    );
    assert!(
        names.contains(&"regular"),
        "Regular property should also be tracked"
    );
}

// ── Class member extraction: getter/setter kinds ────────────

#[test]
fn class_getter_setter_are_class_method_kind() {
    let info = parse_source(
        r"export class Config {
            get value() { return this._value; }
            set value(v: string) { this._value = v; }
            normal() {}
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let value_members: Vec<_> = info.exports[0]
        .members
        .iter()
        .filter(|m| m.name == "value")
        .collect();
    assert!(
        !value_members.is_empty(),
        "Getter/setter 'value' should be present"
    );
    assert!(
        value_members
            .iter()
            .all(|m| m.kind == MemberKind::ClassMethod),
        "Getter/setter should have ClassMethod kind"
    );
    let normal = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "normal")
        .unwrap();
    assert_eq!(normal.kind, MemberKind::ClassMethod);
}

// ── Class member extraction: decorated property ─────────────

#[test]
fn class_decorated_property_with_column_decorator() {
    let info = parse_source(
        r"export class Entity {
            @Column()
            name: string = '';
            age: number = 0;
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let name_member = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "name")
        .expect("name should be in members");
    assert!(
        name_member.has_decorator,
        "@Column() decorated member should have has_decorator = true"
    );
    assert_eq!(name_member.kind, MemberKind::ClassProperty);
    let age_member = info.exports[0]
        .members
        .iter()
        .find(|m| m.name == "age")
        .expect("age should be in members");
    assert!(
        !age_member.has_decorator,
        "Undecorated member should have has_decorator = false"
    );
}

// ── Instance member tracking via new expression ─────────────

#[test]
fn instance_member_access_via_new_expression() {
    let info = parse_source(
        r"import { MyService } from './service';
        const svc = new MyService();
        svc.greet();
        svc.initialize();",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "MyService" && a.member == "greet"),
        "svc.greet() should be mapped to MyService.greet, found: {:?}",
        info.member_accesses
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "MyService" && a.member == "initialize"),
        "svc.initialize() should be mapped to MyService.initialize, found: {:?}",
        info.member_accesses
    );
}

// ── Builtin constructor not tracked ─────────────────────────

#[test]
fn builtin_constructor_instance_not_tracked() {
    let info = parse_source(
        r"const arr = new Array();
        arr.push(1);
        const url = new URL('https://example.com');
        url.hostname;",
    );
    assert!(
        !info.member_accesses.iter().any(|a| a.object == "Array"),
        "new Array() should not create instance binding for member tracking"
    );
    assert!(
        !info.member_accesses.iter().any(|a| a.object == "URL"),
        "new URL() should not create instance binding for member tracking"
    );
}

// ── require.context with regex pattern ──────────────────────

#[test]
fn require_context_with_json_regex() {
    let info = parse_source(r"const ctx = require.context('./locale', false, /\.json$/);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./locale/");
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".json".to_string())
    );
}

// ── Dynamic import string concatenation patterns ────────────

#[test]
fn dynamic_import_concat_prefix_only() {
    let info = parse_source("const m = import('./pages/' + name);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./pages/");
    assert!(
        info.dynamic_import_patterns[0].suffix.is_none(),
        "Concat with only prefix and variable should have no suffix"
    );
}

#[test]
fn dynamic_import_concat_prefix_and_suffix() {
    let info = parse_source("const m = import('./views/' + name + '.vue');");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./views/");
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".vue".to_string())
    );
}

// ── Class with mixed accessibility and decorators ───────────

#[test]
fn class_mixed_members_comprehensive() {
    let info = parse_source(
        r"export class Service {
            static version = '1.0';
            @Inject()
            private db: Database;
            protected logger: Logger;
            public name: string = '';
            constructor(db: Database) { this.db = db; }
            private connect() {}
            protected log() {}
            handle() {}
            @Get('/health')
            healthCheck() {}
        }",
    );
    assert_eq!(info.exports.len(), 1);
    let members = &info.exports[0].members;
    let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();

    // Public and static members included
    assert!(
        names.contains(&"version"),
        "Static property should be included"
    );
    assert!(
        names.contains(&"name"),
        "Public property should be included"
    );
    assert!(
        names.contains(&"handle"),
        "Public method should be included"
    );
    assert!(
        names.contains(&"healthCheck"),
        "Decorated public method should be included"
    );

    // Private, protected, and constructor excluded
    assert!(
        !names.contains(&"db"),
        "Private property should be excluded"
    );
    assert!(
        !names.contains(&"logger"),
        "Protected property should be excluded"
    );
    assert!(
        !names.contains(&"constructor"),
        "Constructor should be excluded"
    );
    assert!(
        !names.contains(&"connect"),
        "Private method should be excluded"
    );
    assert!(
        !names.contains(&"log"),
        "Protected method should be excluded"
    );

    // Decorator tracking
    let health_check = members.iter().find(|m| m.name == "healthCheck").unwrap();
    assert!(
        health_check.has_decorator,
        "healthCheck should have has_decorator = true"
    );
    let handle = members.iter().find(|m| m.name == "handle").unwrap();
    assert!(
        !handle.has_decorator,
        "handle should have has_decorator = false"
    );
}
