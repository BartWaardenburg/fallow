// Visitor tests invoke Oxc parser which is ~1000x slower under Miri.
#![cfg(all(test, not(miri)))]

use std::path::Path;

use super::*;
use crate::tests::parse_ts as parse;
use crate::{ImportedName, MemberKind};
use fallow_types::discover::FileId;
use helpers::regex_pattern_to_suffix;

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
    let source_type = oxc_span::SourceType::from_path(Path::new("extra.ts")).unwrap_or_default();
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
        r"
            export class MyService {
                name: string;
                getValue() { return 1; }
            }
            ",
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
        r"
            export class Foo {
                constructor() {}
                doWork() {}
            }
            ",
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
        r"
            export class Foo {
                private secret: string;
                protected internal(): void {}
                public visible: number;
            }
            ",
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
        r"
            function Injectable() { return (target: any) => target; }
            export class Service {
                @Injectable()
                handler() {}
            }
            ",
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
        r"
            export enum Direction {
                Up,
                Down,
                Left,
                Right
            }
            ",
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
        r"
            export class Foo {
                bar: number;
                baz() { return this.bar; }
            }
            ",
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
        r"
            export class Foo {
                bar: number;
                init() { this.bar = 42; }
            }
            ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "this" && a.member == "bar"),
        "this.bar = ... should be tracked as a member access"
    );
}

// ── Instance member access tracking ─────────────────────────

#[test]
fn instance_member_access_mapped_to_class() {
    let info = parse(
        r"
            import { MyService } from './service';
            const svc = new MyService();
            svc.greet();
            ",
    );
    // svc.greet() should produce a MemberAccess for MyService.greet
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "MyService" && a.member == "greet"),
        "svc.greet() should be mapped to MyService.greet, found: {:?}",
        info.member_accesses
    );
}

#[test]
fn instance_property_access_mapped_to_class() {
    let info = parse(
        r"
            import { MyClass } from './class';
            const obj = new MyClass();
            console.log(obj.name);
            ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "MyClass" && a.member == "name"),
        "obj.name should be mapped to MyClass.name, found: {:?}",
        info.member_accesses
    );
}

#[test]
fn instance_whole_object_use_mapped_to_class() {
    let info = parse(
        r"
            import { MyClass } from './class';
            const obj = new MyClass();
            Object.keys(obj);
            ",
    );
    assert!(
        info.whole_object_uses.contains(&"MyClass".to_string()),
        "Object.keys(obj) should map to whole-object use of MyClass, found: {:?}",
        info.whole_object_uses
    );
}

#[test]
fn non_instance_binding_not_mapped() {
    let info = parse(
        r"
            const obj = { greet() {} };
            obj.greet();
            ",
    );
    // obj is not a `new` binding, so no class mapping should exist.
    assert!(
        !info
            .member_accesses
            .iter()
            .any(|a| { a.object != "obj" && a.object != "this" && a.object != "console" }),
        "non-instance bindings should not produce class-mapped accesses, found: {:?}",
        info.member_accesses
    );
}

#[test]
fn instance_binding_with_no_access_produces_nothing() {
    let info = parse(
        r"
            import { Foo } from './foo';
            const x = new Foo();
            ",
    );
    // Binding exists but no x.method() calls — no synthetic accesses should be emitted.
    assert!(
        !info.member_accesses.iter().any(|a| a.object == "Foo"),
        "binding with no member access should not produce Foo entries, found: {:?}",
        info.member_accesses
    );
    assert!(
        !info.whole_object_uses.contains(&"Foo".to_string()),
        "binding with no whole-object use should not produce Foo entries, found: {:?}",
        info.whole_object_uses
    );
}

#[test]
fn builtin_constructor_not_tracked() {
    let info = parse(
        r"
            const url = new URL('https://example.com');
            url.href;
            const m = new Map();
            m.get('key');
            ",
    );
    // Built-in constructors should not create instance bindings
    assert!(
        !info.member_accesses.iter().any(|a| a.object == "URL"),
        "new URL() should not create instance binding, found: {:?}",
        info.member_accesses
    );
    assert!(
        !info.member_accesses.iter().any(|a| a.object == "Map"),
        "new Map() should not create instance binding, found: {:?}",
        info.member_accesses
    );
}

#[test]
fn multiple_instances_same_class() {
    let info = parse(
        r"
            import { Svc } from './svc';
            const a = new Svc();
            const b = new Svc();
            a.foo();
            b.bar();
            ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Svc" && a.member == "foo"),
        "a.foo() should map to Svc.foo, found: {:?}",
        info.member_accesses
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Svc" && a.member == "bar"),
        "b.bar() should map to Svc.bar, found: {:?}",
        info.member_accesses
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
    let info =
        parse("import { MyEnum } from './types';\nfor (const key in MyEnum) { console.log(key); }");
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

// ── Export extraction ────────────────────────────────────────

#[test]
fn export_default_class_declaration() {
    let info = parse("export default class Foo { bar() {} }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_default_anonymous_class() {
    let info = parse("export default class {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_default_expression() {
    let info = parse("export default 42;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_default_arrow_function() {
    let info = parse("export default () => {};");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Default);
}

#[test]
fn export_const_multiple_declarators() {
    let info = parse("export const a = 1, b = 2, c = 3;");
    assert_eq!(info.exports.len(), 3);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "a"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "b"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "c"))
    );
}

#[test]
fn export_let_declaration() {
    let info = parse("export let mutable = 'hello';");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("mutable".to_string())
    );
    assert!(!info.exports[0].is_type_only);
}

#[test]
fn export_destructured_object() {
    let info = parse("export const { a, b } = { a: 1, b: 2 };");
    assert_eq!(info.exports.len(), 2);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "a"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "b"))
    );
}

#[test]
fn export_destructured_with_default_value() {
    let info = parse("export const { x = 10, y } = obj;");
    assert_eq!(info.exports.len(), 2);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "x"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "y"))
    );
}

#[test]
fn export_destructured_array() {
    let info = parse("export const [first, , third] = [1, 2, 3];");
    assert_eq!(info.exports.len(), 2);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "first"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "third"))
    );
}

#[test]
fn export_specifier_with_alias() {
    let info = parse("const x = 1;\nexport { x as myAlias };");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("myAlias".to_string())
    );
    assert_eq!(info.exports[0].local_name, Some("x".to_string()));
}

#[test]
fn export_specifier_list_multiple() {
    let info = parse("const a = 1; const b = 2; const c = 3;\nexport { a, b, c };");
    assert_eq!(info.exports.len(), 3);
}

#[test]
fn export_async_function() {
    let info = parse("export async function fetchData() {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("fetchData".to_string())
    );
}

#[test]
fn export_generator_function() {
    let info = parse("export function* gen() { yield 1; }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("gen".to_string()));
}

// ── Type exports ─────────────────────────────────────────────

#[test]
fn export_type_alias() {
    let info = parse("export type ID = string | number;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("ID".to_string()));
    assert!(info.exports[0].is_type_only);
}

#[test]
fn export_interface() {
    let info = parse("export interface Props { name: string; }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("Props".to_string()));
    assert!(info.exports[0].is_type_only);
}

#[test]
fn export_type_specifier_on_individual_spec() {
    let info = parse("const a = 1; type B = string;\nexport { a, type B };");
    assert_eq!(info.exports.len(), 2);
    let a_export = info
        .exports
        .iter()
        .find(|e| matches!(&e.name, ExportName::Named(n) if n == "a"))
        .unwrap();
    let b_export = info
        .exports
        .iter()
        .find(|e| matches!(&e.name, ExportName::Named(n) if n == "B"))
        .unwrap();
    assert!(!a_export.is_type_only);
    assert!(b_export.is_type_only);
}

#[test]
fn export_declare_module() {
    let info = parse("export declare module 'my-module' {}");
    assert_eq!(info.exports.len(), 1);
    assert!(info.exports[0].is_type_only);
}

#[test]
fn export_declare_namespace() {
    let info = parse("export declare namespace MyNS {}");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("MyNS".to_string()));
    assert!(info.exports[0].is_type_only);
}

// ── Re-export extraction ─────────────────────────────────────

#[test]
fn re_export_named() {
    let info = parse("export { foo } from './bar';");
    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.re_exports[0].imported_name, "foo");
    assert_eq!(info.re_exports[0].exported_name, "foo");
    assert_eq!(info.re_exports[0].source, "./bar");
}

#[test]
fn re_export_with_rename() {
    let info = parse("export { foo as bar } from './baz';");
    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.re_exports[0].imported_name, "foo");
    assert_eq!(info.re_exports[0].exported_name, "bar");
}

#[test]
fn re_export_multiple() {
    let info = parse("export { a, b, c } from './mod';");
    assert_eq!(info.re_exports.len(), 3);
}

#[test]
fn re_export_star() {
    let info = parse("export * from './all';");
    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.re_exports[0].imported_name, "*");
    assert_eq!(info.re_exports[0].exported_name, "*");
    assert!(!info.re_exports[0].is_type_only);
}

#[test]
fn re_export_star_as_namespace() {
    let info = parse("export * as ns from './all';");
    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.re_exports[0].imported_name, "*");
    assert_eq!(info.re_exports[0].exported_name, "ns");
}

#[test]
fn re_export_type_only() {
    let info = parse("export type { Foo, Bar } from './types';");
    assert_eq!(info.re_exports.len(), 2);
    assert!(info.re_exports[0].is_type_only);
    assert!(info.re_exports[1].is_type_only);
}

#[test]
fn re_export_type_on_individual_specifier() {
    let info = parse("export { type Foo, bar } from './mod';");
    assert_eq!(info.re_exports.len(), 2);
    let foo_re = info
        .re_exports
        .iter()
        .find(|r| r.exported_name == "Foo")
        .unwrap();
    let bar_re = info
        .re_exports
        .iter()
        .find(|r| r.exported_name == "bar")
        .unwrap();
    assert!(foo_re.is_type_only);
    assert!(!bar_re.is_type_only);
}

#[test]
fn re_export_star_type_only() {
    let info = parse("export type * from './types';");
    assert_eq!(info.re_exports.len(), 1);
    assert!(info.re_exports[0].is_type_only);
    assert_eq!(info.re_exports[0].imported_name, "*");
}

// ── Import extraction ────────────────────────────────────────

#[test]
fn import_named_single() {
    let info = parse("import { foo } from './bar';");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(
        info.imports[0].imported_name,
        ImportedName::Named("foo".to_string())
    );
    assert_eq!(info.imports[0].local_name, "foo");
    assert_eq!(info.imports[0].source, "./bar");
}

#[test]
fn import_named_multiple() {
    let info = parse("import { a, b, c } from './mod';");
    assert_eq!(info.imports.len(), 3);
}

#[test]
fn import_default() {
    let info = parse("import React from 'react';");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.imports[0].imported_name, ImportedName::Default);
    assert_eq!(info.imports[0].local_name, "React");
}

#[test]
fn import_namespace() {
    let info = parse("import * as utils from './utils';");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.imports[0].imported_name, ImportedName::Namespace);
    assert_eq!(info.imports[0].local_name, "utils");
}

#[test]
fn import_side_effect() {
    let info = parse("import './styles.css';");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.imports[0].imported_name, ImportedName::SideEffect);
    assert!(info.imports[0].local_name.is_empty());
}

#[test]
fn import_with_alias() {
    let info = parse("import { foo as bar } from './mod';");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(
        info.imports[0].imported_name,
        ImportedName::Named("foo".to_string())
    );
    assert_eq!(info.imports[0].local_name, "bar");
}

#[test]
fn import_default_and_named() {
    let info = parse("import React, { useState, useEffect } from 'react';");
    assert_eq!(info.imports.len(), 3);
    assert_eq!(info.imports[0].imported_name, ImportedName::Default);
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
fn import_default_and_namespace() {
    let info = parse("import def, * as ns from './mod';");
    assert_eq!(info.imports.len(), 2);
    assert_eq!(info.imports[0].imported_name, ImportedName::Default);
    assert_eq!(info.imports[1].imported_name, ImportedName::Namespace);
}

#[test]
fn import_type_only_declaration() {
    let info = parse("import type { Foo } from './types';");
    assert_eq!(info.imports.len(), 1);
    assert!(info.imports[0].is_type_only);
    assert_eq!(
        info.imports[0].imported_name,
        ImportedName::Named("Foo".to_string())
    );
}

#[test]
fn import_type_on_individual_specifier() {
    let info = parse("import { type Foo, Bar } from './types';");
    assert_eq!(info.imports.len(), 2);
    let foo_imp = info.imports.iter().find(|i| i.local_name == "Foo").unwrap();
    let bar_imp = info.imports.iter().find(|i| i.local_name == "Bar").unwrap();
    assert!(foo_imp.is_type_only);
    assert!(!bar_imp.is_type_only);
}

#[test]
fn import_type_namespace() {
    let info = parse("import type * as Types from './types';");
    assert_eq!(info.imports.len(), 1);
    assert!(info.imports[0].is_type_only);
    assert_eq!(info.imports[0].imported_name, ImportedName::Namespace);
}

#[test]
fn import_type_default() {
    let info = parse("import type React from 'react';");
    assert_eq!(info.imports.len(), 1);
    assert!(info.imports[0].is_type_only);
    assert_eq!(info.imports[0].imported_name, ImportedName::Default);
}

#[test]
fn import_source_span_populated() {
    let info = parse("import { foo } from './bar';");
    assert_eq!(info.imports.len(), 1);
    // source_span should cover the string literal './bar'
    assert!(info.imports[0].source_span.start < info.imports[0].source_span.end);
}

// ── Dynamic import extraction ────────────────────────────────

#[test]
fn dynamic_import_string_literal() {
    let info = parse("import('./lazy');");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./lazy");
    assert!(info.dynamic_imports[0].local_name.is_none());
    assert!(info.dynamic_imports[0].destructured_names.is_empty());
}

#[test]
fn dynamic_import_assigned_to_variable() {
    let info = parse("const mod = import('./lazy');");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./lazy");
    assert_eq!(info.dynamic_imports[0].local_name, Some("mod".to_string()));
}

#[test]
fn dynamic_import_await() {
    let info = parse("async function f() { const mod = await import('./lazy'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./lazy");
    assert_eq!(info.dynamic_imports[0].local_name, Some("mod".to_string()));
}

#[test]
fn dynamic_import_destructured() {
    let info = parse("async function f() { const { a, b } = await import('./mod'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert!(info.dynamic_imports[0].local_name.is_none());
    assert_eq!(info.dynamic_imports[0].destructured_names, vec!["a", "b"]);
}

#[test]
fn dynamic_import_destructured_with_rest_clears_names() {
    let info = parse("async function f() { const { a, ...rest } = await import('./mod'); }");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert!(info.dynamic_imports[0].destructured_names.is_empty());
}

#[test]
fn dynamic_import_variable_source_ignored() {
    let info = parse("import(variable);");
    assert!(info.dynamic_imports.is_empty());
    assert!(info.dynamic_import_patterns.is_empty());
}

#[test]
fn dynamic_import_template_literal_exact() {
    let info = parse("import(`./exact`);");
    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./exact");
}

#[test]
fn dynamic_import_template_literal_with_expression() {
    let info = parse("import(`./locales/${lang}.json`);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./locales/");
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".json".to_string())
    );
}

#[test]
fn dynamic_import_template_multi_expression_globstar() {
    let info = parse("import(`./plugins/${cat}/${name}.js`);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./plugins/**/");
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".js".to_string())
    );
}

#[test]
fn dynamic_import_concat_prefix_only() {
    let info = parse("import('./pages/' + name);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./pages/");
    assert!(info.dynamic_import_patterns[0].suffix.is_none());
}

#[test]
fn dynamic_import_concat_with_suffix() {
    let info = parse("import('./pages/' + name + '.tsx');");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./pages/");
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".tsx".to_string())
    );
}

#[test]
fn dynamic_import_non_relative_template_ignored() {
    let info = parse("import(`lodash/${fn}`);");
    assert!(info.dynamic_import_patterns.is_empty());
}

#[test]
fn dynamic_import_non_relative_concat_ignored() {
    let info = parse("import('lodash/' + fn);");
    assert!(info.dynamic_import_patterns.is_empty());
}

#[test]
fn dynamic_import_no_duplicate_when_assigned() {
    // When assigned to a variable, the import should appear exactly once
    let info = parse("async function f() { const m = await import('./svc'); }");
    assert_eq!(
        info.dynamic_imports.len(),
        1,
        "assigned dynamic import should not produce duplicate entries"
    );
}

// ── Require call extraction ──────────────────────────────────

#[test]
fn require_call_simple() {
    let info = parse("const fs = require('fs');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(info.require_calls[0].source, "fs");
    assert_eq!(info.require_calls[0].local_name, Some("fs".to_string()));
}

#[test]
fn require_call_destructured() {
    let info = parse("const { readFile, writeFile } = require('fs');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(info.require_calls[0].source, "fs");
    assert!(info.require_calls[0].local_name.is_none());
    assert_eq!(
        info.require_calls[0].destructured_names,
        vec!["readFile", "writeFile"]
    );
}

#[test]
fn require_call_bare_in_expression() {
    let info = parse("doSomething(require('foo'));");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(info.require_calls[0].source, "foo");
    assert!(info.require_calls[0].local_name.is_none());
}

#[test]
fn require_call_variable_arg_ignored() {
    let info = parse("const x = require(someVar);");
    assert!(info.require_calls.is_empty());
}

#[test]
fn require_call_template_literal_arg_ignored() {
    let info = parse("const x = require(`./mod`);");
    assert!(info.require_calls.is_empty());
}

#[test]
fn require_multiple_calls() {
    let info = parse("const a = require('a'); const b = require('b');");
    assert_eq!(info.require_calls.len(), 2);
}

#[test]
fn require_destructured_with_alias() {
    let info = parse("const { foo: localFoo } = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert_eq!(info.require_calls[0].destructured_names, vec!["foo"]);
}

#[test]
fn require_destructured_with_rest_returns_empty() {
    let info = parse("const { a, ...rest } = require('./mod');");
    assert_eq!(info.require_calls.len(), 1);
    assert!(info.require_calls[0].destructured_names.is_empty());
}

// ── Member access extraction ─────────────────────────────────

#[test]
fn member_access_static() {
    let info = parse("import { Status } from './types';\nStatus.Active;");
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Status" && a.member == "Active"),
        "should track Status.Active"
    );
}

#[test]
fn member_access_method_call() {
    let info = parse("import { MyClass } from './mod';\nMyClass.create();");
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "MyClass" && a.member == "create"),
        "should track MyClass.create"
    );
}

#[test]
fn member_access_computed_string_literal() {
    let info = parse("import { Status } from './types';\nStatus['Active'];");
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Status" && a.member == "Active"),
        "computed access with string literal should resolve to member"
    );
}

#[test]
fn member_access_computed_dynamic_marks_whole() {
    let info = parse("import { Status } from './types';\nconst k = 'x';\nStatus[k];");
    assert!(
        info.whole_object_uses.contains(&"Status".to_string()),
        "dynamic computed access should mark as whole-object use"
    );
}

#[test]
fn member_access_this_read() {
    let info = parse(
        r"
        export class Foo {
            x: number;
            getX() { return this.x; }
        }
        ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "this" && a.member == "x"),
        "this.x read should be tracked"
    );
}

#[test]
fn member_access_this_write() {
    let info = parse(
        r"
        export class Foo {
            x: number;
            setX() { this.x = 5; }
        }
        ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "this" && a.member == "x"),
        "this.x = ... should be tracked"
    );
}

#[test]
fn member_access_chained() {
    let info = parse("import { obj } from './data';\nobj.a.b.c;");
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "obj" && a.member == "a"),
        "first level of chained access should be tracked"
    );
}

// ── Whole-object use patterns ────────────────────────────────

#[test]
fn whole_object_object_values() {
    let info = parse("Object.values(myObj);");
    assert!(info.whole_object_uses.contains(&"myObj".to_string()));
}

#[test]
fn whole_object_object_keys() {
    let info = parse("Object.keys(myObj);");
    assert!(info.whole_object_uses.contains(&"myObj".to_string()));
}

#[test]
fn whole_object_object_entries() {
    let info = parse("Object.entries(myObj);");
    assert!(info.whole_object_uses.contains(&"myObj".to_string()));
}

#[test]
fn whole_object_get_own_property_names() {
    let info = parse("Object.getOwnPropertyNames(myObj);");
    assert!(info.whole_object_uses.contains(&"myObj".to_string()));
}

#[test]
fn whole_object_spread() {
    let info = parse("const copy = { ...myObj };");
    assert!(info.whole_object_uses.contains(&"myObj".to_string()));
}

#[test]
fn whole_object_for_in() {
    let info = parse("for (const k in myObj) {}");
    assert!(info.whole_object_uses.contains(&"myObj".to_string()));
}

#[test]
fn whole_object_spread_in_array() {
    let info = parse("const arr = [...myArr];");
    assert!(info.whole_object_uses.contains(&"myArr".to_string()));
}

#[test]
fn whole_object_spread_in_call_args() {
    let info = parse("fn(...myArr);");
    assert!(info.whole_object_uses.contains(&"myArr".to_string()));
}

// ── CommonJS exports ─────────────────────────────────────────

#[test]
fn cjs_module_exports_object_keys() {
    let info = parse("module.exports = { foo: 1, bar: 2, baz: 3 };");
    assert!(info.has_cjs_exports);
    assert_eq!(info.exports.len(), 3);
}

#[test]
fn cjs_exports_dot_property() {
    let info = parse("exports.myFunc = function() {};");
    assert!(info.has_cjs_exports);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "myFunc"))
    );
}

#[test]
fn cjs_module_exports_non_object() {
    let info = parse("module.exports = someValue;");
    assert!(info.has_cjs_exports);
    // Non-object RHS doesn't produce named exports
    assert!(info.exports.is_empty());
}

#[test]
fn cjs_both_patterns() {
    let info = parse("module.exports = { a: 1 };\nexports.b = 2;");
    assert!(info.has_cjs_exports);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "a"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "b"))
    );
}

#[test]
fn cjs_module_exports_dot_property() {
    let info = parse(
        "module.exports.foo = function() {};\nmodule.exports.bar = 42;\nmodule.exports.baz = class {};",
    );
    assert!(info.has_cjs_exports);
    assert_eq!(info.exports.len(), 3);
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
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "baz"))
    );
}

// ── TypeScript enum extraction ───────────────────────────────

#[test]
fn ts_enum_members_extracted() {
    let info = parse("export enum Color { Red, Green, Blue }");
    assert_eq!(info.exports.len(), 1);
    let members = &info.exports[0].members;
    assert_eq!(members.len(), 3);
    assert!(members.iter().all(|m| m.kind == MemberKind::EnumMember));
    assert!(members.iter().any(|m| m.name == "Red"));
    assert!(members.iter().any(|m| m.name == "Green"));
    assert!(members.iter().any(|m| m.name == "Blue"));
}

#[test]
fn ts_enum_with_string_values() {
    let info = parse(r#"export enum Status { Active = "active", Inactive = "inactive" }"#);
    assert_eq!(info.exports.len(), 1);
    let members = &info.exports[0].members;
    assert_eq!(members.len(), 2);
    assert!(members.iter().any(|m| m.name == "Active"));
    assert!(members.iter().any(|m| m.name == "Inactive"));
}

#[test]
fn ts_enum_with_numeric_values() {
    let info = parse("export enum Dir { Up = 0, Down = 1, Left = 2, Right = 3 }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 4);
}

#[test]
fn ts_const_enum() {
    let info = parse("export const enum Flags { A, B, C }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 3);
}

#[test]
fn ts_enum_string_member_name() {
    let info = parse(r#"export enum E { "some-key" = 1 }"#);
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 1);
    assert_eq!(info.exports[0].members[0].name, "some-key");
}

// ── Class member extraction ──────────────────────────────────

#[test]
fn class_public_methods_and_properties() {
    let info = parse(
        r"
        export class Svc {
            name: string;
            greet() {}
            static create() {}
        }
        ",
    );
    let class_export = info
        .exports
        .iter()
        .find(|e| matches!(&e.name, ExportName::Named(n) if n == "Svc"))
        .unwrap();
    assert!(
        class_export
            .members
            .iter()
            .any(|m| m.name == "name" && m.kind == MemberKind::ClassProperty)
    );
    assert!(
        class_export
            .members
            .iter()
            .any(|m| m.name == "greet" && m.kind == MemberKind::ClassMethod)
    );
    assert!(
        class_export
            .members
            .iter()
            .any(|m| m.name == "create" && m.kind == MemberKind::ClassMethod)
    );
}

#[test]
fn class_skips_constructor() {
    let info = parse("export class Foo { constructor() {} }");
    let members = &info.exports[0].members;
    assert!(!members.iter().any(|m| m.name == "constructor"));
}

#[test]
fn class_skips_private_members() {
    let info = parse(
        r"
        export class Foo {
            private secret: string;
            public visible: number;
        }
        ",
    );
    let members = &info.exports[0].members;
    assert!(!members.iter().any(|m| m.name == "secret"));
    assert!(members.iter().any(|m| m.name == "visible"));
}

#[test]
fn class_skips_protected_members() {
    let info = parse(
        r"
        export class Foo {
            protected internal(): void {}
            open(): void {}
        }
        ",
    );
    let members = &info.exports[0].members;
    assert!(!members.iter().any(|m| m.name == "internal"));
    assert!(members.iter().any(|m| m.name == "open"));
}

#[test]
fn class_member_decorator_tracked() {
    let info = parse(
        r"
        function Dec() { return (t: any) => t; }
        export class Svc {
            @Dec()
            handler() {}
            plain() {}
        }
        ",
    );
    let members = &info.exports[0].members;
    let handler = members.iter().find(|m| m.name == "handler").unwrap();
    let plain = members.iter().find(|m| m.name == "plain").unwrap();
    assert!(handler.has_decorator);
    assert!(!plain.has_decorator);
}

// ── Instance member access mapping ───────────────────────────

#[test]
fn instance_method_call_mapped() {
    let info = parse(
        r"
        import { MyService } from './svc';
        const svc = new MyService();
        svc.hello();
        ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "MyService" && a.member == "hello")
    );
}

#[test]
fn instance_property_mapped() {
    let info = parse(
        r"
        import { Config } from './config';
        const cfg = new Config();
        console.log(cfg.port);
        ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Config" && a.member == "port")
    );
}

#[test]
fn builtin_constructor_instance_not_mapped() {
    let info = parse(
        r"
        const m = new Map();
        m.set('key', 'value');
        ",
    );
    assert!(
        !info.member_accesses.iter().any(|a| a.object == "Map"),
        "built-in Map should not produce instance mapping"
    );
}

#[test]
fn instance_whole_object_mapped() {
    let info = parse(
        r"
        import { MyClass } from './cls';
        const obj = new MyClass();
        Object.keys(obj);
        ",
    );
    assert!(info.whole_object_uses.contains(&"MyClass".to_string()));
}

#[test]
fn multiple_instances_same_class_mapped() {
    let info = parse(
        r"
        import { Svc } from './svc';
        const a = new Svc();
        const b = new Svc();
        a.foo();
        b.bar();
        ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Svc" && a.member == "foo")
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "Svc" && a.member == "bar")
    );
}

// ── Namespace destructuring ──────────────────────────────────

#[test]
fn namespace_import_destructuring() {
    let info = parse("import * as ns from './mod';\nconst { a, b } = ns;");
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "ns" && a.member == "a")
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "ns" && a.member == "b")
    );
}

#[test]
fn namespace_import_destructuring_with_rest_marks_whole() {
    let info = parse("import * as ns from './mod';\nconst { a, ...rest } = ns;");
    assert!(info.whole_object_uses.contains(&"ns".to_string()));
}

#[test]
fn require_namespace_destructuring() {
    let info = parse("const mod = require('./mod');\nconst { x, y } = mod;");
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "mod" && a.member == "x")
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "mod" && a.member == "y")
    );
}

#[test]
fn dynamic_import_namespace_destructuring() {
    let info = parse(
        r"
        async function f() {
            const mod = await import('./mod');
            const { foo, bar } = mod;
        }
        ",
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "mod" && a.member == "foo")
    );
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "mod" && a.member == "bar")
    );
}

#[test]
fn non_namespace_destructuring_not_tracked() {
    let info = parse("const obj = { a: 1 };\nconst { a } = obj;");
    assert!(
        !info
            .member_accesses
            .iter()
            .any(|a| a.object == "obj" && a.member == "a"),
        "destructuring of non-namespace vars should not produce member accesses"
    );
}

// ── new URL pattern ──────────────────────────────────────────

#[test]
fn new_url_import_meta_url_tracked() {
    let info = parse("new URL('./worker.js', import.meta.url);");
    assert!(
        info.dynamic_imports
            .iter()
            .any(|d| d.source == "./worker.js")
    );
}

#[test]
fn new_url_non_relative_not_tracked() {
    let info = parse("new URL('https://example.com', import.meta.url);");
    assert!(info.dynamic_imports.is_empty());
}

#[test]
fn new_url_without_import_meta_url_not_tracked() {
    let info = parse("new URL('./worker.js', baseUrl);");
    assert!(info.dynamic_imports.is_empty());
}

// ── import.meta.glob ─────────────────────────────────────────

#[test]
fn import_meta_glob_string() {
    let info = parse("import.meta.glob('./components/*.tsx');");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./components/*.tsx");
}

#[test]
fn import_meta_glob_array() {
    let info = parse("import.meta.glob(['./a/*.ts', './b/*.ts']);");
    assert_eq!(info.dynamic_import_patterns.len(), 2);
}

#[test]
fn import_meta_glob_non_relative_ignored() {
    let info = parse("import.meta.glob('node_modules/**/*.js');");
    assert!(info.dynamic_import_patterns.is_empty());
}

// ── require.context ──────────────────────────────────────────

#[test]
fn require_context_non_recursive_prefix() {
    let info = parse("require.context('./icons', false);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./icons/");
}

#[test]
fn require_context_recursive_prefix() {
    let info = parse("require.context('./icons', true);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(info.dynamic_import_patterns[0].prefix, "./icons/**/");
}

#[test]
fn require_context_with_regex_suffix() {
    let info = parse(r"require.context('./src', true, /\.vue$/);");
    assert_eq!(info.dynamic_import_patterns.len(), 1);
    assert_eq!(
        info.dynamic_import_patterns[0].suffix,
        Some(".vue".to_string())
    );
}

#[test]
fn require_context_non_relative_ignored() {
    let info = parse("require.context('node_modules', false);");
    assert!(info.dynamic_import_patterns.is_empty());
}

// ── Function overload deduplication ──────────────────────────

#[test]
fn function_overloads_produce_single_export() {
    let info = parse(
        r"
        export function parse(): void;
        export function parse(input: string): void;
        export function parse(input?: string): void {}
        ",
    );
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("parse".to_string()));
}

// ── Edge cases ───────────────────────────────────────────────

#[test]
fn empty_source_produces_no_results() {
    let info = parse("");
    assert!(info.exports.is_empty());
    assert!(info.imports.is_empty());
    assert!(info.re_exports.is_empty());
    assert!(info.dynamic_imports.is_empty());
    assert!(info.require_calls.is_empty());
    assert!(!info.has_cjs_exports);
}

#[test]
fn no_module_syntax_produces_no_results() {
    let info = parse("const x = 1;\nconsole.log(x);");
    assert!(info.exports.is_empty());
    assert!(info.imports.is_empty());
    assert!(info.re_exports.is_empty());
    assert!(!info.has_cjs_exports);
}

#[test]
fn namespace_import_adds_to_namespace_bindings() {
    let info = parse("import * as ns from './mod';\nns.foo();");
    // ns should be tracked as a namespace binding and ns.foo as a member access
    assert!(
        info.member_accesses
            .iter()
            .any(|a| a.object == "ns" && a.member == "foo")
    );
}

#[test]
fn export_abstract_class() {
    let info = parse("export abstract class Base { abstract doWork(): void; }");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, ExportName::Named("Base".to_string()));
}

#[test]
fn export_enum_not_type_only() {
    let info = parse("export enum Dir { Up, Down }");
    assert_eq!(info.exports.len(), 1);
    assert!(!info.exports[0].is_type_only);
}

#[test]
fn mixed_esm_and_cjs_in_same_file() {
    let info =
        parse("import { foo } from './bar';\nexport const x = 1;\nmodule.exports = { y: 2 };");
    assert_eq!(info.imports.len(), 1);
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "x"))
    );
    assert!(
        info.exports
            .iter()
            .any(|e| matches!(&e.name, ExportName::Named(n) if n == "y"))
    );
    assert!(info.has_cjs_exports);
}

#[test]
fn export_with_satisfies() {
    let info = parse("export const config = { port: 3000 } satisfies Config;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("config".to_string())
    );
}

#[test]
fn export_with_as_const() {
    let info = parse("export const COLORS = ['red', 'blue'] as const;");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        ExportName::Named("COLORS".to_string())
    );
}

#[test]
fn import_and_re_export_same_source() {
    let info = parse("import { foo } from './mod';\nexport { bar } from './mod';");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.imports[0].source, "./mod");
    assert_eq!(info.re_exports[0].source, "./mod");
}

mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_valid_js_source() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("const x = 1;".to_string()),
            Just("export const a = 42;".to_string()),
            Just("import { foo } from './bar';".to_string()),
            Just("export default function() {}".to_string()),
            Just("export { x } from './mod';".to_string()),
            Just("const y = require('./util');".to_string()),
            Just("export class Foo {}".to_string()),
            Just("export type T = string;".to_string()),
            Just("export interface I { x: number; }".to_string()),
            Just("import * as ns from './all';".to_string()),
            Just("export * from './barrel';".to_string()),
            "[a-zA-Z_][a-zA-Z0-9_]{0,20}".prop_map(|id| format!("export const {id} = 1;")),
            "[a-zA-Z_][a-zA-Z0-9_]{0,20}".prop_map(|id| format!("import {{ {id} }} from './mod';")),
        ]
    }

    proptest! {
        /// Parsing any valid JS/TS source never panics.
        #[test]
        fn parse_never_panics(source in "[a-zA-Z0-9 (){};=+\\-*/'\",.<>:\\n!?@#$%^&|~`_]{0,200}") {
            let _ = parse(&source);
        }

        /// Star re-export sources should go into re_exports, not exports.
        #[test]
        fn star_reexport_does_not_pollute_exports(
            mod_name in "[a-z]{1,10}",
        ) {
            let source = format!("export * from './{mod_name}';");
            let info = parse(&source);
            // Star re-exports should be in re_exports, not exports
            prop_assert!(
                !info.re_exports.is_empty(),
                "Star re-export should produce a re_export entry"
            );
            // The export list should not contain the re-exported names
            for exp in &info.exports {
                if let ExportName::Named(name) = &exp.name {
                    prop_assert_ne!(name, "*", "Star re-export should not appear in exports");
                }
            }
        }

        /// All export names should be non-empty strings (for Named variants).
        #[test]
        fn export_names_are_non_empty(source in arb_valid_js_source()) {
            let info = parse(&source);
            for export in &info.exports {
                if let ExportName::Named(name) = &export.name {
                    prop_assert!(!name.is_empty(), "Named export should have non-empty name");
                }
            }
        }

        /// All import sources should be non-empty strings.
        #[test]
        fn import_sources_are_non_empty(source in arb_valid_js_source()) {
            let info = parse(&source);
            for import in &info.imports {
                prop_assert!(!import.source.is_empty(), "Import source should be non-empty");
            }
            for re_export in &info.re_exports {
                prop_assert!(!re_export.source.is_empty(), "Re-export source should be non-empty");
            }
        }
    }
}
