// Visitor tests invoke Oxc parser which is ~1000x slower under Miri.
#![cfg(all(test, not(miri)))]

use super::*;
use crate::parse::parse_source_to_module;
use fallow_types::discover::FileId;
use helpers::regex_pattern_to_suffix;
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

// ── Instance member access tracking ─────────────────────────

#[test]
fn instance_member_access_mapped_to_class() {
    let info = parse(
        r#"
            import { MyService } from './service';
            const svc = new MyService();
            svc.greet();
            "#,
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
        r#"
            import { MyClass } from './class';
            const obj = new MyClass();
            console.log(obj.name);
            "#,
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
        r#"
            import { MyClass } from './class';
            const obj = new MyClass();
            Object.keys(obj);
            "#,
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
        r#"
            const obj = { greet() {} };
            obj.greet();
            "#,
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
        r#"
            import { Foo } from './foo';
            const x = new Foo();
            "#,
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
        r#"
            const url = new URL('https://example.com');
            url.href;
            const m = new Map();
            m.get('key');
            "#,
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
        r#"
            import { Svc } from './svc';
            const a = new Svc();
            const b = new Svc();
            a.foo();
            b.bar();
            "#,
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
