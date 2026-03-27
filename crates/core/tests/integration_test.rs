#[path = "integration_test/common.rs"]
mod common;

#[path = "integration_test/barrel_exports.rs"]
mod barrel_exports;
#[path = "integration_test/basic_analysis.rs"]
mod basic_analysis;
#[path = "integration_test/caching.rs"]
mod caching;
#[path = "integration_test/css_modules.rs"]
mod css_modules;
#[path = "integration_test/dependencies.rs"]
mod dependencies;
#[path = "integration_test/duplicates.rs"]
mod duplicates;
#[path = "integration_test/dynamic_imports.rs"]
mod dynamic_imports;
#[path = "integration_test/external_plugins.rs"]
mod external_plugins;
#[path = "integration_test/extraction.rs"]
mod extraction;
#[path = "integration_test/frameworks.rs"]
mod frameworks;
#[path = "integration_test/member_detection.rs"]
mod member_detection;
#[path = "integration_test/rules_config.rs"]
mod rules_config;
#[path = "integration_test/sfc_parsing.rs"]
mod sfc_parsing;
#[path = "integration_test/workspaces.rs"]
mod workspaces;

#[path = "integration_test/css_modules_unused.rs"]
mod css_modules_unused;
#[path = "integration_test/production_mode.rs"]
mod production_mode;
#[path = "integration_test/re_export_chains.rs"]
mod re_export_chains;
#[path = "integration_test/suppression_comments.rs"]
mod suppression_comments;
#[path = "integration_test/type_only_deps.rs"]
mod type_only_deps;
#[path = "integration_test/unused_enum_members.rs"]
mod unused_enum_members;
#[path = "integration_test/workspace_cross_imports.rs"]
mod workspace_cross_imports;
