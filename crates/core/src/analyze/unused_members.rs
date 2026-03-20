use rustc_hash::{FxHashMap, FxHashSet};

use fallow_config::ResolvedConfig;

use crate::discover::FileId;
use crate::extract::MemberKind;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;
use crate::suppress::{self, IssueKind, Suppression};

use super::predicates::{is_angular_lifecycle_method, is_react_lifecycle_method};
use super::{byte_offset_to_line_col, read_source};

/// Find unused enum and class members in exported symbols.
///
/// Collects all `Identifier.member` static member accesses from all modules,
/// maps them to their imported names, and filters out members that are accessed.
pub(crate) fn find_unused_members(
    graph: &ModuleGraph,
    _config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
    suppressions_by_file: &FxHashMap<FileId, &[Suppression]>,
) -> (Vec<UnusedMember>, Vec<UnusedMember>) {
    let mut unused_enum_members = Vec::new();
    let mut unused_class_members = Vec::new();

    // Map export_name -> set of member_names that are accessed across all modules.
    // We map local import names back to the original imported names.
    let mut accessed_members: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();

    // Also build a per-file set of `this.member` accesses. These indicate internal usage
    // within a class body — class members accessed via `this.foo` are used internally
    // even if no external code accesses them via `ClassName.foo`.
    let mut self_accessed_members: FxHashMap<crate::discover::FileId, FxHashSet<String>> =
        FxHashMap::default();

    // Build a set of export names that are used as whole objects (Object.values, for..in, etc.).
    // All members of these exports should be considered used.
    let mut whole_object_used_exports: FxHashSet<String> = FxHashSet::default();

    for resolved in resolved_modules {
        // Build a map from local name -> imported name for this module's imports
        let local_to_imported: FxHashMap<&str, &str> = resolved
            .resolved_imports
            .iter()
            .filter_map(|imp| match &imp.info.imported_name {
                crate::extract::ImportedName::Named(name) => {
                    Some((imp.info.local_name.as_str(), name.as_str()))
                }
                crate::extract::ImportedName::Default => {
                    Some((imp.info.local_name.as_str(), "default"))
                }
                _ => None,
            })
            .collect();

        for access in &resolved.member_accesses {
            // Track `this.member` accesses per-file for internal class usage
            if access.object == "this" {
                self_accessed_members
                    .entry(resolved.file_id)
                    .or_default()
                    .insert(access.member.clone());
                continue;
            }
            // If the object is a local name for an import, map it to the original export name
            let export_name = local_to_imported
                .get(access.object.as_str())
                .copied()
                .unwrap_or(access.object.as_str());
            accessed_members
                .entry(export_name.to_string())
                .or_default()
                .insert(access.member.clone());
        }

        // Map whole-object uses from local names to imported names
        for local_name in &resolved.whole_object_uses {
            let export_name = local_to_imported
                .get(local_name.as_str())
                .copied()
                .unwrap_or(local_name.as_str());
            whole_object_used_exports.insert(export_name.to_string());
        }
    }

    for module in &graph.modules {
        if !module.is_reachable || module.is_entry_point {
            continue;
        }

        // Lazily load source content for line/col computation
        let mut source_content: Option<String> = None;

        for export in &module.exports {
            if export.members.is_empty() {
                continue;
            }

            // If the export itself is unused, skip member analysis (whole export is dead)
            if export.references.is_empty() && !graph.has_namespace_import(module.file_id) {
                continue;
            }

            let export_name = export.name.to_string();

            // If this export is used as a whole object (Object.values, for..in, etc.),
            // all members are considered used — skip individual member analysis.
            if whole_object_used_exports.contains(&export_name) {
                continue;
            }

            // Get `this.member` accesses from this file (internal class usage)
            let file_self_accesses = self_accessed_members.get(&module.file_id);

            for member in &export.members {
                // Check if this member is accessed anywhere via external import
                if accessed_members
                    .get(&export_name)
                    .is_some_and(|s| s.contains(&member.name))
                {
                    continue;
                }

                // Check if this member is accessed via `this.member` within the same file
                // (internal class usage — e.g., constructor sets this.label, methods use this.label)
                if matches!(
                    member.kind,
                    MemberKind::ClassMethod | MemberKind::ClassProperty
                ) && file_self_accesses.is_some_and(|accesses| accesses.contains(&member.name))
                {
                    continue;
                }

                // Skip decorated class members — decorators like @Column(), @ApiProperty(),
                // @Inject() etc. indicate runtime usage by frameworks (NestJS, TypeORM,
                // class-validator, class-transformer). These members are accessed
                // reflectively and should never be flagged as unused.
                if member.has_decorator {
                    continue;
                }

                // Skip React class component lifecycle methods — they are called by the
                // React runtime, not user code, so they should never be flagged as unused.
                // Also skip Angular lifecycle hooks (OnInit, OnDestroy, etc.).
                if matches!(
                    member.kind,
                    MemberKind::ClassMethod | MemberKind::ClassProperty
                ) && (is_react_lifecycle_method(&member.name)
                    || is_angular_lifecycle_method(&member.name))
                {
                    continue;
                }

                let source = source_content.get_or_insert_with(|| read_source(&module.path));
                let (line, col) = byte_offset_to_line_col(source, member.span.start);

                // Check inline suppression
                let issue_kind = match member.kind {
                    MemberKind::EnumMember => IssueKind::UnusedEnumMember,
                    MemberKind::ClassMethod | MemberKind::ClassProperty => {
                        IssueKind::UnusedClassMember
                    }
                };
                if let Some(supps) = suppressions_by_file.get(&module.file_id)
                    && suppress::is_suppressed(supps, line, issue_kind)
                {
                    continue;
                }

                let unused = UnusedMember {
                    path: module.path.clone(),
                    parent_name: export_name.clone(),
                    member_name: member.name.clone(),
                    kind: member.kind.clone(),
                    line,
                    col,
                };

                match member.kind {
                    MemberKind::EnumMember => unused_enum_members.push(unused),
                    MemberKind::ClassMethod | MemberKind::ClassProperty => {
                        unused_class_members.push(unused);
                    }
                }
            }
        }
    }

    (unused_enum_members, unused_class_members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use crate::extract::{
        ExportName, ImportInfo, ImportedName, MemberAccess, MemberInfo, MemberKind,
    };
    use crate::graph::{ExportSymbol, ModuleGraph, SymbolReference};
    use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};
    use oxc_span::Span;
    use std::path::PathBuf;

    fn build_graph(file_specs: &[(&str, bool)]) -> ModuleGraph {
        let files: Vec<DiscoveredFile> = file_specs
            .iter()
            .enumerate()
            .map(|(i, (path, _))| DiscoveredFile {
                id: FileId(i as u32),
                path: PathBuf::from(path),
                size_bytes: 0,
            })
            .collect();

        let entry_points: Vec<EntryPoint> = file_specs
            .iter()
            .filter(|(_, is_entry)| *is_entry)
            .map(|(path, _)| EntryPoint {
                path: PathBuf::from(path),
                source: EntryPointSource::ManualEntry,
            })
            .collect();

        let resolved_modules: Vec<ResolvedModule> = files
            .iter()
            .map(|f| ResolvedModule {
                file_id: f.id,
                path: f.path.clone(),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            })
            .collect();

        ModuleGraph::build(&resolved_modules, &entry_points, &files)
    }

    fn test_config() -> ResolvedConfig {
        fallow_config::FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(
            PathBuf::from("/tmp/test"),
            fallow_config::OutputFormat::Human,
            1,
            true,
        )
    }

    fn make_member(name: &str, kind: MemberKind) -> MemberInfo {
        MemberInfo {
            name: name.to_string(),
            kind,
            span: Span::new(10, 20),
            has_decorator: false,
        }
    }

    fn make_export_with_members(
        name: &str,
        members: Vec<MemberInfo>,
        ref_from: Option<u32>,
    ) -> ExportSymbol {
        let references = ref_from
            .map(|from| {
                vec![SymbolReference {
                    from_file: FileId(from),
                    kind: crate::graph::ReferenceKind::NamedImport,
                    import_span: Span::new(0, 10),
                }]
            })
            .unwrap_or_default();
        ExportSymbol {
            name: ExportName::Named(name.to_string()),
            is_type_only: false,
            span: Span::new(0, 10),
            references,
            members,
        }
    }

    #[test]
    fn unused_members_empty_graph() {
        let graph = build_graph(&[]);
        let config = test_config();
        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert!(enum_members.is_empty());
        assert!(class_members.is_empty());
    }

    #[test]
    fn unused_enum_member_detected() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/enums.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "Status",
            vec![
                make_member("Active", MemberKind::EnumMember),
                make_member("Inactive", MemberKind::EnumMember),
            ],
            Some(0), // referenced from entry
        )];
        let config = test_config();

        // No member accesses at all — both should be unused
        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert_eq!(enum_members.len(), 2);
        assert!(class_members.is_empty());
        let names: FxHashSet<&str> = enum_members
            .iter()
            .map(|m| m.member_name.as_str())
            .collect();
        assert!(names.contains("Active"));
        assert!(names.contains("Inactive"));
    }

    #[test]
    fn accessed_enum_member_not_flagged() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/enums.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "Status",
            vec![
                make_member("Active", MemberKind::EnumMember),
                make_member("Inactive", MemberKind::EnumMember),
            ],
            Some(0),
        )];
        let config = test_config();

        // Consumer accesses Status.Active
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/src/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "./enums".to_string(),
                    imported_name: ImportedName::Named("Status".to_string()),
                    local_name: "Status".to_string(),
                    is_type_only: false,
                    span: Span::new(0, 30),
                },
                target: ResolveResult::InternalModule(FileId(1)),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![MemberAccess {
                object: "Status".to_string(),
                member: "Active".to_string(),
            }],
            whole_object_uses: vec![],
            has_cjs_exports: false,
        }];

        let (enum_members, _) =
            find_unused_members(&graph, &config, &resolved_modules, &FxHashMap::default());
        // Only Inactive should be unused
        assert_eq!(enum_members.len(), 1);
        assert_eq!(enum_members[0].member_name, "Inactive");
    }

    #[test]
    fn whole_object_use_skips_all_members() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/enums.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "Status",
            vec![
                make_member("Active", MemberKind::EnumMember),
                make_member("Inactive", MemberKind::EnumMember),
            ],
            Some(0),
        )];
        let config = test_config();

        // Consumer uses Object.values(Status) — whole object use
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/src/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "./enums".to_string(),
                    imported_name: ImportedName::Named("Status".to_string()),
                    local_name: "Status".to_string(),
                    is_type_only: false,
                    span: Span::new(0, 30),
                },
                target: ResolveResult::InternalModule(FileId(1)),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec!["Status".to_string()],
            has_cjs_exports: false,
        }];

        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &resolved_modules, &FxHashMap::default());
        assert!(enum_members.is_empty());
        assert!(class_members.is_empty());
    }

    #[test]
    fn decorated_class_member_not_flagged() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/entity.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "User",
            vec![MemberInfo {
                name: "name".to_string(),
                kind: MemberKind::ClassProperty,
                span: Span::new(10, 20),
                has_decorator: true, // @Column() etc.
            }],
            Some(0),
        )];
        let config = test_config();

        let (_, class_members) = find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert!(class_members.is_empty());
    }

    #[test]
    fn react_lifecycle_method_not_flagged() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/component.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "MyComponent",
            vec![
                make_member("render", MemberKind::ClassMethod),
                make_member("componentDidMount", MemberKind::ClassMethod),
                make_member("customMethod", MemberKind::ClassMethod),
            ],
            Some(0),
        )];
        let config = test_config();

        let (_, class_members) = find_unused_members(&graph, &config, &[], &FxHashMap::default());
        // Only customMethod should be flagged
        assert_eq!(class_members.len(), 1);
        assert_eq!(class_members[0].member_name, "customMethod");
    }

    #[test]
    fn angular_lifecycle_method_not_flagged() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/component.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "AppComponent",
            vec![
                make_member("ngOnInit", MemberKind::ClassMethod),
                make_member("ngOnDestroy", MemberKind::ClassMethod),
                make_member("myHelper", MemberKind::ClassMethod),
            ],
            Some(0),
        )];
        let config = test_config();

        let (_, class_members) = find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert_eq!(class_members.len(), 1);
        assert_eq!(class_members[0].member_name, "myHelper");
    }

    #[test]
    fn this_member_access_not_flagged() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/service.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "Service",
            vec![
                make_member("label", MemberKind::ClassProperty),
                make_member("unused_prop", MemberKind::ClassProperty),
            ],
            Some(0),
        )];
        let config = test_config();

        // The service file itself accesses this.label
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(1), // same file as the service
            path: PathBuf::from("/src/service.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![MemberAccess {
                object: "this".to_string(),
                member: "label".to_string(),
            }],
            whole_object_uses: vec![],
            has_cjs_exports: false,
        }];

        let (_, class_members) =
            find_unused_members(&graph, &config, &resolved_modules, &FxHashMap::default());
        // Only unused_prop should be flagged (label is accessed via this)
        assert_eq!(class_members.len(), 1);
        assert_eq!(class_members[0].member_name, "unused_prop");
    }

    #[test]
    fn unreferenced_export_skips_member_analysis() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/enums.ts", false)]);
        graph.modules[1].is_reachable = true;
        // Export has members but NO references — whole export is dead, members skipped
        graph.modules[1].exports = vec![make_export_with_members(
            "Status",
            vec![make_member("Active", MemberKind::EnumMember)],
            None, // no references
        )];
        let config = test_config();

        let (enum_members, _) = find_unused_members(&graph, &config, &[], &FxHashMap::default());
        // Member analysis skipped because export itself is unreferenced
        assert!(enum_members.is_empty());
    }

    #[test]
    fn unreachable_module_skips_member_analysis() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/dead.ts", false)]);
        // Module 1 stays unreachable
        graph.modules[1].exports = vec![make_export_with_members(
            "DeadEnum",
            vec![make_member("X", MemberKind::EnumMember)],
            Some(0),
        )];
        let config = test_config();

        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert!(enum_members.is_empty());
        assert!(class_members.is_empty());
    }

    #[test]
    fn entry_point_module_skips_member_analysis() {
        let mut graph = build_graph(&[("/src/entry.ts", true)]);
        graph.modules[0].exports = vec![make_export_with_members(
            "EntryEnum",
            vec![make_member("X", MemberKind::EnumMember)],
            None,
        )];
        let config = test_config();

        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert!(enum_members.is_empty());
        assert!(class_members.is_empty());
    }

    #[test]
    fn enum_member_kind_routed_to_enum_results() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/enums.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "Status",
            vec![make_member("Active", MemberKind::EnumMember)],
            Some(0),
        )];
        let config = test_config();

        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert_eq!(enum_members.len(), 1);
        assert_eq!(enum_members[0].kind, MemberKind::EnumMember);
        assert!(class_members.is_empty());
    }

    #[test]
    fn class_member_kind_routed_to_class_results() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/class.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export_with_members(
            "MyClass",
            vec![
                make_member("myMethod", MemberKind::ClassMethod),
                make_member("myProp", MemberKind::ClassProperty),
            ],
            Some(0),
        )];
        let config = test_config();

        let (enum_members, class_members) =
            find_unused_members(&graph, &config, &[], &FxHashMap::default());
        assert!(enum_members.is_empty());
        assert_eq!(class_members.len(), 2);
        assert!(
            class_members
                .iter()
                .any(|m| m.kind == MemberKind::ClassMethod)
        );
        assert!(
            class_members
                .iter()
                .any(|m| m.kind == MemberKind::ClassProperty)
        );
    }
}
