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
    assert_eq!(info.require_calls[0].destructured_names, vec!["a", "b", "c"]);
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
    let info = parse_source(
        "async function f() { const { foo: f1, bar: b1 } = await import('./mod'); }",
    );
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
