//! Parsing and extraction engine for the fallow dead code analyzer.
//!
//! This crate handles all file parsing: JS/TS via Oxc, Vue/Svelte SFC extraction,
//! Astro frontmatter, MDX import/export extraction, CSS Module class name extraction,
//! and incremental caching of parse results.

pub mod astro;
pub mod cache;
pub mod css;
pub mod mdx;
mod parse;
pub mod sfc;
pub mod suppress;
pub mod visitor;

use std::path::Path;

use rayon::prelude::*;

use cache::CacheStore;
use fallow_types::discover::{DiscoveredFile, FileId};

// Re-export all extract types from fallow-types
pub use fallow_types::extract::{
    DynamicImportInfo, DynamicImportPattern, ExportInfo, ExportName, ImportInfo, ImportedName,
    MemberAccess, MemberInfo, MemberKind, ModuleInfo, ParseResult, ReExportInfo, RequireCallInfo,
};

// Re-export extraction functions for internal use and fuzzing
pub use astro::extract_astro_frontmatter;
pub use css::extract_css_module_exports;
pub use mdx::extract_mdx_statements;
pub use sfc::{extract_sfc_scripts, is_sfc_file};

use parse::parse_source_to_module;

/// Parse all files in parallel, extracting imports and exports.
/// Uses the cache to skip reparsing files whose content hasn't changed.
pub fn parse_all_files(files: &[DiscoveredFile], cache: Option<&CacheStore>) -> ParseResult {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let cache_hits = AtomicUsize::new(0);
    let cache_misses = AtomicUsize::new(0);

    let modules: Vec<ModuleInfo> = files
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

    ParseResult {
        modules,
        cache_hits: hits,
        cache_misses: misses,
    }
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
        return Some(cache::cached_to_module(cached, file.id));
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

/// Parse from in-memory content (for LSP).
pub fn parse_from_content(file_id: FileId, path: &Path, content: &str) -> ModuleInfo {
    let content_hash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
    parse_source_to_module(file_id, path, content, content_hash)
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

    #[test]
    fn vue_script_lang_tsx() {
        let info = parse_sfc(
            r#"
<script lang="tsx">
import { defineComponent } from 'vue';
export default defineComponent({
    render() { return <div>Hello</div>; }
});
</script>
"#,
            "TsxVue.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn svelte_context_module_script() {
        let info = parse_sfc(
            r#"
<script context="module" lang="ts">
export const preload = () => {};
</script>
<script lang="ts">
import { onMount } from 'svelte';
let count = 0;
</script>
"#,
            "Module.svelte",
        );
        assert!(info.imports.iter().any(|i| i.source == "svelte"));
        assert!(!info.exports.is_empty());
    }

    #[test]
    fn vue_script_with_generic_attr() {
        let info = parse_sfc(
            r#"
<script setup lang="ts" generic="T extends Record<string, unknown>">
import { ref } from 'vue';
const items = ref<T[]>([]);
</script>
"#,
            "Generic.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn vue_empty_script_block() {
        let info = parse_sfc(
            r#"<script lang="ts"></script><template><div/></template>"#,
            "Empty.vue",
        );
        assert!(info.imports.is_empty());
        assert!(info.exports.is_empty());
    }

    #[test]
    fn vue_whitespace_only_script() {
        let info = parse_sfc(
            "<script lang=\"ts\">\n  \n</script>\n<template><div/></template>",
            "Whitespace.vue",
        );
        assert!(info.imports.is_empty());
    }

    #[test]
    fn vue_script_src_attribute() {
        let info = parse_sfc(
            r#"<script src="./component.ts" lang="ts"></script><template><div/></template>"#,
            "External.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./component.ts");
    }

    #[test]
    fn vue_script_inside_html_comment() {
        let info = parse_sfc(
            r#"
<!-- <script lang="ts">
import { bad } from 'should-not-be-found';
</script> -->
<script lang="ts">
import { good } from 'vue';
</script>
<template><div/></template>
"#,
            "Commented.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn vue_script_setup_with_compiler_macros() {
        let info = parse_sfc(
            r#"
<script setup lang="ts">
import { ref } from 'vue';
const props = defineProps<{ msg: string }>();
const emit = defineEmits<{ change: [value: string] }>();
const count = ref(0);
</script>
"#,
            "Macros.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn vue_script_with_single_quoted_lang() {
        let info = parse_sfc(
            "<script lang='ts'>\nimport { ref } from 'vue';\n</script>",
            "SingleQuote.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn svelte_generics_attribute() {
        let info = parse_sfc(
            r#"
<script lang="ts" generics="T extends Record<string, unknown>">
import { onMount } from 'svelte';
export let items: T[] = [];
</script>
"#,
            "Generic.svelte",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "svelte");
    }

    #[test]
    fn vue_script_with_extra_attributes() {
        let info = parse_sfc(
            r#"
<script lang="ts" id="app-script" type="module" data-custom="value">
import { ref } from 'vue';
</script>
"#,
            "ExtraAttrs.vue",
        );
        assert_eq!(info.imports.len(), 1);
    }

    #[test]
    fn vue_multiple_script_setup_invalid() {
        let info = parse_sfc(
            r#"
<script setup lang="ts">
import { ref } from 'vue';
</script>
<script setup lang="ts">
import { computed } from 'vue';
</script>
"#,
            "DuplicateSetup.vue",
        );
        assert!(info.imports.len() >= 2);
    }

    #[test]
    fn vue_script_case_insensitive() {
        let info = parse_sfc(
            "<SCRIPT lang=\"ts\">\nimport { ref } from 'vue';\n</SCRIPT>",
            "Upper.vue",
        );
        assert_eq!(info.imports.len(), 1);
    }

    #[test]
    fn svelte_script_with_context_and_generics() {
        let info = parse_sfc(
            r#"
<script context="module" lang="ts">
export function preload() { return {}; }
</script>
<script lang="ts" generics="T">
import { onMount } from 'svelte';
export let value: T;
</script>
"#,
            "ContextGenerics.svelte",
        );
        assert!(info.imports.iter().any(|i| i.source == "svelte"));
        assert!(!info.exports.is_empty());
    }

    #[test]
    fn vue_script_with_nested_generics() {
        let info = parse_sfc(
            r#"
<script setup lang="ts" generic="T extends Map<string, Set<number>>">
import { ref } from 'vue';
const items = ref<T>();
</script>
"#,
            "NestedGeneric.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn vue_script_src_with_body_ignored() {
        let info = parse_sfc(
            r#"<script src="./external.ts" lang="ts">
import { unused } from 'should-not-matter';
</script>"#,
            "SrcWithBody.vue",
        );
        assert!(info.imports.iter().any(|i| i.source == "./external.ts"));
    }

    #[test]
    fn vue_data_src_not_treated_as_src() {
        let info = parse_sfc(
            r#"<script lang="ts" data-src="./not-a-module.ts">
import { ref } from 'vue';
</script>"#,
            "DataSrc.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn vue_html_comment_string_not_corrupted() {
        let info = parse_sfc(
            r#"
<script setup lang="ts">
const htmlComment = "<!-- this is not a comment -->";
import { ref } from 'vue';
</script>
"#,
            "CommentString.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    #[test]
    fn vue_script_spanning_html_comment() {
        let info = parse_sfc(
            r#"
<!-- disabled:
<script lang="ts">
import { bad } from 'should-not-be-found';
</script>
-->
<script lang="ts">
import { good } from 'vue';
</script>
"#,
            "SpanningComment.vue",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "vue");
    }

    // ── Astro frontmatter parsing ──────────────────────────────

    #[test]
    fn extracts_astro_frontmatter_imports() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("Layout.astro"),
            r#"---
import Layout from '../layouts/Layout.astro';
import { Card } from '../components/Card';
const title = "Hello";
---
<Layout title={title}>
  <Card />
</Layout>
"#,
            0,
        );
        assert_eq!(info.imports.len(), 2);
        assert!(
            info.imports
                .iter()
                .any(|i| i.source == "../layouts/Layout.astro")
        );
        assert!(
            info.imports
                .iter()
                .any(|i| i.source == "../components/Card")
        );
    }

    #[test]
    fn astro_no_frontmatter_returns_empty() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("Simple.astro"),
            "<div>No frontmatter here</div>",
            0,
        );
        assert!(info.imports.is_empty());
        assert!(info.exports.is_empty());
    }

    #[test]
    fn astro_empty_frontmatter() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("Empty.astro"),
            "---\n---\n<div>Content</div>",
            0,
        );
        assert!(info.imports.is_empty());
    }

    #[test]
    fn astro_frontmatter_with_dynamic_import() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("Dynamic.astro"),
            r#"---
const mod = await import('../utils/helper');
---
<div>{mod.value}</div>
"#,
            0,
        );
        assert_eq!(info.dynamic_imports.len(), 1);
        assert_eq!(info.dynamic_imports[0].source, "../utils/helper");
    }

    #[test]
    fn astro_frontmatter_with_reexport() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("ReExport.astro"),
            r#"---
export { default as Layout } from '../layouts/Layout.astro';
---
<div>Content</div>
"#,
            0,
        );
        assert_eq!(info.re_exports.len(), 1);
    }

    // ── MDX import extraction ──────────────────────────────────

    #[test]
    fn extracts_mdx_imports() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("post.mdx"),
            r#"import { Chart } from './Chart'
import Button from './Button'

# My Post

Some markdown content here.

<Chart data={[1, 2, 3]} />
<Button>Click me</Button>
"#,
            0,
        );
        assert_eq!(info.imports.len(), 2);
        assert!(info.imports.iter().any(|i| i.source == "./Chart"));
        assert!(info.imports.iter().any(|i| i.source == "./Button"));
    }

    #[test]
    fn extracts_mdx_exports() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("post.mdx"),
            r#"export const meta = { title: 'Hello' }

# My Post

Content here.
"#,
            0,
        );
        assert!(!info.exports.is_empty());
    }

    #[test]
    fn mdx_no_imports_returns_empty() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("simple.mdx"),
            "# Just Markdown\n\nNo imports here.\n",
            0,
        );
        assert!(info.imports.is_empty());
        assert!(info.exports.is_empty());
    }

    #[test]
    fn mdx_multiline_import() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("multi.mdx"),
            r#"import {
  Chart,
  Table,
  Graph
} from './components'

# Dashboard

<Chart />
"#,
            0,
        );
        assert_eq!(info.imports.len(), 3);
        assert!(info.imports.iter().all(|i| i.source == "./components"));
    }

    #[test]
    fn mdx_imports_between_content() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("mixed.mdx"),
            r#"import { Header } from './Header'

# Section 1

Some content.

import { Footer } from './Footer'

## Section 2

More content.
"#,
            0,
        );
        assert_eq!(info.imports.len(), 2);
        assert!(info.imports.iter().any(|i| i.source == "./Header"));
        assert!(info.imports.iter().any(|i| i.source == "./Footer"));
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

    // ── Dynamic import namespace tracking ────────────────────────

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
        let info = parse_source(
            "async function f() { const { foo, ...rest } = await import('./module'); }",
        );
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

    // ---- CSS/SCSS extraction tests ----

    fn parse_css(source: &str, filename: &str) -> ModuleInfo {
        parse_source_to_module(FileId(0), Path::new(filename), source, 0)
    }

    #[test]
    fn extracts_css_import_quoted() {
        let info = parse_css(r#"@import "./reset.css";"#, "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./reset.css");
        assert_eq!(info.imports[0].imported_name, ImportedName::SideEffect);
    }

    #[test]
    fn extracts_css_import_single_quoted() {
        let info = parse_css("@import './variables.css';", "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./variables.css");
    }

    #[test]
    fn extracts_css_import_url() {
        let info = parse_css(r#"@import url("./base.css");"#, "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./base.css");
    }

    #[test]
    fn extracts_css_import_url_single_quoted() {
        let info = parse_css("@import url('./base.css');", "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./base.css");
    }

    #[test]
    fn extracts_css_import_url_unquoted() {
        let info = parse_css("@import url(./base.css);", "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./base.css");
    }

    #[test]
    fn extracts_multiple_css_imports() {
        let info = parse_css(
            r#"
@import "./reset.css";
@import "./variables.css";
@import url("./base.css");
"#,
            "styles.css",
        );
        assert_eq!(info.imports.len(), 3);
        assert_eq!(info.imports[0].source, "./reset.css");
        assert_eq!(info.imports[1].source, "./variables.css");
        assert_eq!(info.imports[2].source, "./base.css");
    }

    #[test]
    fn extracts_css_import_tailwind_package() {
        let info = parse_css(r#"@import "tailwindcss";"#, "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "tailwindcss");
    }

    #[test]
    fn css_apply_creates_tailwind_dependency() {
        let info = parse_css(
            r#"
.btn {
    @apply px-4 py-2 bg-blue-500 text-white;
}
"#,
            "styles.css",
        );
        assert!(
            info.imports.iter().any(|i| i.source == "tailwindcss"),
            "should create synthetic tailwindcss import"
        );
    }

    #[test]
    fn css_tailwind_directive_creates_dependency() {
        let info = parse_css(
            r#"
@tailwind base;
@tailwind components;
@tailwind utilities;
"#,
            "styles.css",
        );
        assert!(
            info.imports.iter().any(|i| i.source == "tailwindcss"),
            "should create synthetic tailwindcss import"
        );
    }

    #[test]
    fn css_without_apply_no_tailwind_dependency() {
        let info = parse_css(
            r#"
.btn {
    padding: 4px;
    color: blue;
}
"#,
            "styles.css",
        );
        assert!(
            !info.imports.iter().any(|i| i.source == "tailwindcss"),
            "should NOT create tailwindcss import without @apply"
        );
    }

    #[test]
    fn extracts_scss_use() {
        let info = parse_css(r#"@use "./variables";"#, "styles.scss");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./variables");
    }

    #[test]
    fn extracts_scss_forward() {
        let info = parse_css(r#"@forward "./mixins";"#, "styles.scss");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./mixins");
    }

    #[test]
    fn scss_use_not_extracted_from_css() {
        let info = parse_css(r#"@use "./variables";"#, "styles.css");
        assert_eq!(info.imports.len(), 0);
    }

    #[test]
    fn css_apply_with_multiple_classes() {
        let info = parse_css(
            r#"
.card {
    @apply shadow-lg rounded-lg p-4;
}
.header {
    @apply text-xl font-bold;
}
"#,
            "styles.css",
        );
        let tw_imports: Vec<_> = info
            .imports
            .iter()
            .filter(|i| i.source == "tailwindcss")
            .collect();
        assert_eq!(tw_imports.len(), 1);
    }

    #[test]
    fn css_file_has_no_exports() {
        let info = parse_css(
            r#"
@import "./reset.css";
.btn { @apply px-4 py-2; }
"#,
            "styles.css",
        );
        assert!(info.exports.is_empty(), "CSS files should not have exports");
        assert!(info.re_exports.is_empty());
    }

    #[test]
    fn scss_combined_imports_and_apply() {
        let info = parse_css(
            r#"
@use "./variables";
@use "./mixins";
@import "./reset.css";

.btn {
    @apply px-4 py-2;
}
"#,
            "app.scss",
        );
        assert_eq!(info.imports.len(), 4);
        assert!(info.imports.iter().any(|i| i.source == "./variables"));
        assert!(info.imports.iter().any(|i| i.source == "./mixins"));
        assert!(info.imports.iter().any(|i| i.source == "./reset.css"));
        assert!(info.imports.iter().any(|i| i.source == "tailwindcss"));
    }

    #[test]
    fn css_import_with_media_query() {
        let info = parse_css(r#"@import "./print.css" print;"#, "styles.css");
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./print.css");
    }

    #[test]
    fn css_commented_apply_not_extracted() {
        let info = parse_css(
            r#"
/* @apply px-4 py-2; */
.btn {
    padding: 4px;
}
"#,
            "styles.css",
        );
        assert!(
            !info.imports.iter().any(|i| i.source == "tailwindcss"),
            "commented-out @apply should NOT create tailwindcss import"
        );
    }

    #[test]
    fn css_commented_import_not_extracted() {
        let info = parse_css(
            r#"
/* @import "./old-reset.css"; */
.btn { color: red; }
"#,
            "styles.css",
        );
        assert!(info.imports.is_empty());
    }

    #[test]
    fn css_commented_tailwind_not_extracted() {
        let info = parse_css(
            r#"
/*
@tailwind base;
@tailwind components;
@tailwind utilities;
*/
.btn { color: red; }
"#,
            "styles.css",
        );
        assert!(
            !info.imports.iter().any(|i| i.source == "tailwindcss"),
            "commented-out @tailwind should NOT create tailwindcss import"
        );
    }

    #[test]
    fn scss_line_comment_not_extracted() {
        let info = parse_css(
            r#"
// @use "./old-variables";
// @apply px-4;
.btn { color: red; }
"#,
            "styles.scss",
        );
        assert!(info.imports.is_empty());
    }

    #[test]
    fn css_url_import_skipped() {
        let info = parse_css(
            r#"
@import "https://fonts.googleapis.com/css?family=Roboto";
@import url("https://cdn.example.com/reset.css");
@import "./local.css";
"#,
            "styles.css",
        );
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].source, "./local.css");
    }

    #[test]
    fn css_data_uri_import_skipped() {
        let info = parse_css(
            r#"@import url("data:text/css;base64,Ym9keSB7fQ==");"#,
            "styles.css",
        );
        assert!(info.imports.is_empty());
    }

    #[test]
    fn css_mixed_comments_and_real_directives() {
        let info = parse_css(
            r#"
/* @import "./commented-out.css"; */
@import "./real-import.css";
/* @apply hidden; */
.visible {
    @apply block text-lg;
}
"#,
            "styles.css",
        );
        assert_eq!(info.imports.len(), 2);
        assert!(info.imports.iter().any(|i| i.source == "./real-import.css"));
        assert!(info.imports.iter().any(|i| i.source == "tailwindcss"));
    }

    // ── CSS Module extraction ─────────────────────────────────────

    fn parse_css_module(source: &str) -> ModuleInfo {
        parse_source_to_module(FileId(0), Path::new("Component.module.css"), source, 0)
    }

    fn parse_css_non_module(source: &str) -> ModuleInfo {
        parse_source_to_module(FileId(0), Path::new("styles.css"), source, 0)
    }

    #[test]
    fn css_module_extracts_class_names_as_exports() {
        let info = parse_css_module(".header { color: red; } .footer { color: blue; }");
        let export_names: Vec<&ExportName> = info.exports.iter().map(|e| &e.name).collect();
        assert!(export_names.contains(&&ExportName::Named("header".to_string())));
        assert!(export_names.contains(&&ExportName::Named("footer".to_string())));
        assert!(!export_names.contains(&&ExportName::Default));
    }

    #[test]
    fn css_module_extracts_kebab_case_class_names() {
        let info = parse_css_module(".nav-bar { display: flex; } .main-content { padding: 10px; }");
        let named: Vec<String> = info
            .exports
            .iter()
            .filter_map(|e| match &e.name {
                ExportName::Named(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        assert!(named.contains(&"nav-bar".to_string()));
        assert!(named.contains(&"main-content".to_string()));
    }

    #[test]
    fn css_module_deduplicates_class_names() {
        let info = parse_css_module(".btn { color: red; } .btn { font-size: 14px; }");
        let named_count = info
            .exports
            .iter()
            .filter(|e| matches!(&e.name, ExportName::Named(n) if n == "btn"))
            .count();
        assert_eq!(
            named_count, 1,
            "Duplicate class names should be deduplicated"
        );
    }

    #[test]
    fn css_module_no_default_export() {
        let info = parse_css_module(".foo { color: red; }");
        assert!(
            !info.exports.iter().any(|e| e.name == ExportName::Default),
            "CSS modules should not emit a default export (handled at graph level)"
        );
    }

    #[test]
    fn non_module_css_has_no_exports() {
        let info = parse_css_non_module(".header { color: red; }");
        assert!(
            info.exports.is_empty(),
            "Non-module CSS should have no exports"
        );
    }

    #[test]
    fn css_module_ignores_classes_in_comments() {
        let info = parse_css_module("/* .commented { color: red; } */ .active { color: green; }");
        let named: Vec<String> = info
            .exports
            .iter()
            .filter_map(|e| match &e.name {
                ExportName::Named(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !named.contains(&"commented".to_string()),
            "Classes in comments should be ignored"
        );
        assert!(named.contains(&"active".to_string()));
    }

    #[test]
    fn scss_module_extracts_class_names() {
        let info = parse_source_to_module(
            FileId(0),
            Path::new("Component.module.scss"),
            ".wrapper { .inner { color: red; } }",
            0,
        );
        let named: Vec<String> = info
            .exports
            .iter()
            .filter_map(|e| match &e.name {
                ExportName::Named(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        assert!(named.contains(&"wrapper".to_string()));
        assert!(named.contains(&"inner".to_string()));
    }

    #[test]
    fn css_module_with_complex_selectors() {
        let info =
            parse_css_module(".btn:hover { color: red; } .btn.active { } .container > .child { }");
        let named: Vec<String> = info
            .exports
            .iter()
            .filter_map(|e| match &e.name {
                ExportName::Named(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        assert!(named.contains(&"btn".to_string()));
        assert!(named.contains(&"active".to_string()));
        assert!(named.contains(&"container".to_string()));
        assert!(named.contains(&"child".to_string()));
    }

    #[test]
    fn css_module_ignores_classes_in_strings_and_urls() {
        let info = parse_css_module(
            r#".real { content: ".fake"; background: url(./img/hero.png); } .also-real { color: red; }"#,
        );
        let named: Vec<String> = info
            .exports
            .iter()
            .filter_map(|e| match &e.name {
                ExportName::Named(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        assert!(named.contains(&"real".to_string()));
        assert!(named.contains(&"also-real".to_string()));
        assert!(
            !named.contains(&"fake".to_string()),
            "Classes inside quoted strings should be ignored"
        );
        assert!(
            !named.contains(&"png".to_string()),
            "File extensions inside url() should be ignored"
        );
    }
}
