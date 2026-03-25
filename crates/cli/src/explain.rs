//! Metric and rule definitions for explainable CLI output.
//!
//! Provides structured metadata that describes what each metric, threshold,
//! and rule means — consumed by the `_meta` object in JSON output and by
//! SARIF `fullDescription` / `helpUri` fields.

use serde_json::{Value, json};

// ── Docs base URL ────────────────────────────────────────────────

const DOCS_BASE: &str = "https://docs.fallow.tools";

/// Docs URL for the check command.
pub const CHECK_DOCS: &str = "https://docs.fallow.tools/cli/check";

/// Docs URL for the health command.
pub const HEALTH_DOCS: &str = "https://docs.fallow.tools/cli/health";

/// Docs URL for the dupes command.
pub const DUPES_DOCS: &str = "https://docs.fallow.tools/cli/dupes";

// ── Check rules ─────────────────────────────────────────────────

/// Rule definition for SARIF `fullDescription` and JSON `_meta`.
pub struct RuleDef {
    pub id: &'static str,
    pub name: &'static str,
    pub short: &'static str,
    pub full: &'static str,
    pub docs_path: &'static str,
}

pub const CHECK_RULES: &[RuleDef] = &[
    RuleDef {
        id: "fallow/unused-file",
        name: "Unused Files",
        short: "File is not reachable from any entry point",
        full: "Source files that are not imported by any other module and are not entry points (scripts, tests, configs). These files can safely be deleted. Detection uses graph reachability from configured entry points.",
        docs_path: "explanations/dead-code#unused-files",
    },
    RuleDef {
        id: "fallow/unused-export",
        name: "Unused Exports",
        short: "Export is never imported",
        full: "Named exports that are never imported by any other module in the project. Includes both direct exports and re-exports through barrel files. The export may still be used locally within the same file.",
        docs_path: "explanations/dead-code#unused-exports",
    },
    RuleDef {
        id: "fallow/unused-type",
        name: "Unused Type Exports",
        short: "Type export is never imported",
        full: "Type-only exports (interfaces, type aliases, enums used only as types) that are never imported. These do not generate runtime code but add maintenance burden.",
        docs_path: "explanations/dead-code#unused-types",
    },
    RuleDef {
        id: "fallow/unused-dependency",
        name: "Unused Dependencies",
        short: "Dependency listed but never imported",
        full: "Packages listed in dependencies that are never imported or required by any source file. Framework plugins and CLI tools may be false positives — use the ignore_dependencies config to suppress.",
        docs_path: "explanations/dead-code#unused-dependencies",
    },
    RuleDef {
        id: "fallow/unused-dev-dependency",
        name: "Unused Dev Dependencies",
        short: "Dev dependency listed but never imported",
        full: "Packages listed in devDependencies that are never imported by test files, config files, or scripts. Build tools and jest presets that are referenced only in config may appear as false positives.",
        docs_path: "explanations/dead-code#unused-devdependencies",
    },
    RuleDef {
        id: "fallow/unused-optional-dependency",
        name: "Unused Optional Dependencies",
        short: "Optional dependency listed but never imported",
        full: "Packages listed in optionalDependencies that are never imported. Optional dependencies are typically platform-specific — verify they are not needed on any supported platform before removing.",
        docs_path: "explanations/dead-code#unused-optionaldependencies",
    },
    RuleDef {
        id: "fallow/type-only-dependency",
        name: "Type-only Dependencies",
        short: "Production dependency only used via type-only imports",
        full: "Production dependencies that are only imported via `import type` statements. These can be moved to devDependencies since they generate no runtime code and are stripped during compilation.",
        docs_path: "explanations/dead-code#type-only-dependencies",
    },
    RuleDef {
        id: "fallow/unused-enum-member",
        name: "Unused Enum Members",
        short: "Enum member is never referenced",
        full: "Enum members that are never referenced in the codebase. Uses scope-aware binding analysis to track all references including computed access patterns.",
        docs_path: "explanations/dead-code#unused-enum-members",
    },
    RuleDef {
        id: "fallow/unused-class-member",
        name: "Unused Class Members",
        short: "Class member is never referenced",
        full: "Class methods and properties that are never referenced outside the class. Private members are checked within the class scope; public members are checked project-wide.",
        docs_path: "explanations/dead-code#unused-class-members",
    },
    RuleDef {
        id: "fallow/unresolved-import",
        name: "Unresolved Imports",
        short: "Import could not be resolved",
        full: "Import specifiers that could not be resolved to a file on disk. Common causes: deleted files, typos in paths, missing path aliases in tsconfig, or uninstalled packages.",
        docs_path: "explanations/dead-code#unresolved-imports",
    },
    RuleDef {
        id: "fallow/unlisted-dependency",
        name: "Unlisted Dependencies",
        short: "Dependency used but not in package.json",
        full: "Packages that are imported in source code but not listed in package.json. These work by accident (hoisted from another workspace package or transitive dep) and will break in strict package managers.",
        docs_path: "explanations/dead-code#unlisted-dependencies",
    },
    RuleDef {
        id: "fallow/duplicate-export",
        name: "Duplicate Exports",
        short: "Export name appears in multiple modules",
        full: "The same export name is defined in multiple modules. Consumers may import from the wrong module, leading to subtle bugs. Consider renaming or consolidating.",
        docs_path: "explanations/dead-code#duplicate-exports",
    },
    RuleDef {
        id: "fallow/circular-dependency",
        name: "Circular Dependencies",
        short: "Circular dependency chain detected",
        full: "A cycle in the module import graph. Circular dependencies cause undefined behavior with CommonJS (partial modules) and initialization ordering issues with ESM. Break cycles by extracting shared code.",
        docs_path: "explanations/dead-code#circular-dependencies",
    },
];

/// Look up a rule definition by its SARIF rule ID across all rule sets.
pub fn rule_by_id(id: &str) -> Option<&'static RuleDef> {
    CHECK_RULES
        .iter()
        .chain(HEALTH_RULES.iter())
        .chain(DUPES_RULES.iter())
        .find(|r| r.id == id)
}

/// Build the docs URL for a rule.
pub fn rule_docs_url(rule: &RuleDef) -> String {
    format!("{DOCS_BASE}/{}", rule.docs_path)
}

// ── Health SARIF rules ──────────────────────────────────────────

pub const HEALTH_RULES: &[RuleDef] = &[
    RuleDef {
        id: "fallow/high-cyclomatic-complexity",
        name: "High Cyclomatic Complexity",
        short: "Function has high cyclomatic complexity",
        full: "McCabe cyclomatic complexity exceeds the configured threshold. Cyclomatic complexity counts the number of independent paths through a function (1 + decision points: if/else, switch cases, loops, ternary, logical operators). High values indicate functions that are hard to test exhaustively.",
        docs_path: "explanations/health#cyclomatic-complexity",
    },
    RuleDef {
        id: "fallow/high-cognitive-complexity",
        name: "High Cognitive Complexity",
        short: "Function has high cognitive complexity",
        full: "SonarSource cognitive complexity exceeds the configured threshold. Unlike cyclomatic complexity, cognitive complexity penalizes nesting depth and non-linear control flow (breaks, continues, early returns). It measures how hard a function is to understand when reading sequentially.",
        docs_path: "explanations/health#cognitive-complexity",
    },
    RuleDef {
        id: "fallow/high-complexity",
        name: "High Complexity (Both)",
        short: "Function exceeds both complexity thresholds",
        full: "Function exceeds both cyclomatic and cognitive complexity thresholds. This is the strongest signal that a function needs refactoring — it has many paths AND is hard to understand.",
        docs_path: "explanations/health#complexity-metrics",
    },
    RuleDef {
        id: "fallow/refactoring-target",
        name: "Refactoring Target",
        short: "File identified as a high-priority refactoring candidate",
        full: "File identified as a refactoring candidate based on a weighted combination of complexity density, churn velocity, dead code ratio, fan-in (blast radius), and fan-out (coupling). Categories: urgent churn+complexity, break circular dependency, split high-impact file, remove dead code, extract complex functions, reduce coupling.",
        docs_path: "explanations/health#refactoring-targets",
    },
];

pub const DUPES_RULES: &[RuleDef] = &[RuleDef {
    id: "fallow/code-duplication",
    name: "Code Duplication",
    short: "Duplicated code block",
    full: "A block of code that appears in multiple locations with identical or near-identical token sequences. Clone detection uses normalized token comparison — identifier names and literals are abstracted away in non-strict modes.",
    docs_path: "explanations/duplication#clone-groups",
}];

// ── JSON _meta builders ─────────────────────────────────────────

/// Build the `_meta` object for `fallow dead-code --format json --explain`.
pub fn check_meta() -> Value {
    let rules: Value = CHECK_RULES
        .iter()
        .map(|r| {
            (
                r.id.replace("fallow/", ""),
                json!({
                    "name": r.name,
                    "description": r.full,
                    "docs": rule_docs_url(r)
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>()
        .into();

    json!({
        "docs": CHECK_DOCS,
        "rules": rules
    })
}

/// Build the `_meta` object for `fallow health --format json --explain`.
pub fn health_meta() -> Value {
    json!({
        "docs": HEALTH_DOCS,
        "metrics": {
            "cyclomatic": {
                "name": "Cyclomatic Complexity",
                "description": "McCabe cyclomatic complexity: 1 + number of decision points (if/else, switch cases, loops, ternary, logical operators). Measures the number of independent paths through a function.",
                "range": "[1, \u{221e})",
                "interpretation": "lower is better; default threshold: 20"
            },
            "cognitive": {
                "name": "Cognitive Complexity",
                "description": "SonarSource cognitive complexity: penalizes nesting depth and non-linear control flow (breaks, continues, early returns). Measures how hard a function is to understand when reading top-to-bottom.",
                "range": "[0, \u{221e})",
                "interpretation": "lower is better; default threshold: 15"
            },
            "line_count": {
                "name": "Line Count",
                "description": "Number of lines in the function body.",
                "range": "[1, \u{221e})",
                "interpretation": "context-dependent; long functions may need splitting"
            },
            "maintainability_index": {
                "name": "Maintainability Index",
                "description": "Composite score: 100 - (complexity_density \u{00d7} 30) - (dead_code_ratio \u{00d7} 20) - min(ln(fan_out+1) \u{00d7} 4, 15). Clamped to [0, 100]. Higher is better.",
                "range": "[0, 100]",
                "interpretation": "higher is better; <40 poor, 40\u{2013}70 moderate, >70 good"
            },
            "complexity_density": {
                "name": "Complexity Density",
                "description": "Total cyclomatic complexity divided by lines of code. Measures how densely complex the code is per line.",
                "range": "[0, \u{221e})",
                "interpretation": "lower is better; >1.0 indicates very dense complexity"
            },
            "dead_code_ratio": {
                "name": "Dead Code Ratio",
                "description": "Fraction of value exports (excluding type-only exports like interfaces and type aliases) with zero references across the project.",
                "range": "[0, 1]",
                "interpretation": "lower is better; 0 = all exports are used"
            },
            "fan_in": {
                "name": "Fan-in (Importers)",
                "description": "Number of files that import this file. High fan-in means high blast radius \u{2014} changes to this file affect many dependents.",
                "range": "[0, \u{221e})",
                "interpretation": "context-dependent; high fan-in files need careful review before changes"
            },
            "fan_out": {
                "name": "Fan-out (Imports)",
                "description": "Number of files this file directly imports. High fan-out indicates high coupling and change propagation risk.",
                "range": "[0, \u{221e})",
                "interpretation": "lower is better; MI penalty caps at ~40 imports"
            },
            "score": {
                "name": "Hotspot Score",
                "description": "normalized_churn \u{00d7} normalized_complexity \u{00d7} 100, where normalization is against the project maximum. Identifies files that are both complex AND frequently changing.",
                "range": "[0, 100]",
                "interpretation": "higher = riskier; prioritize refactoring high-score files"
            },
            "weighted_commits": {
                "name": "Weighted Commits",
                "description": "Recency-weighted commit count using exponential decay with 90-day half-life. Recent commits contribute more than older ones.",
                "range": "[0, \u{221e})",
                "interpretation": "higher = more recent churn activity"
            },
            "trend": {
                "name": "Churn Trend",
                "description": "Compares recent vs older commit frequency within the analysis window. accelerating = recent > 1.5\u{00d7} older, cooling = recent < 0.67\u{00d7} older, stable = in between.",
                "values": ["accelerating", "stable", "cooling"],
                "interpretation": "accelerating files need attention; cooling files are stabilizing"
            },
            "priority": {
                "name": "Refactoring Priority",
                "description": "Weighted score: complexity density (30%), hotspot boost (25%), dead code ratio (20%), fan-in (15%), fan-out (10%). Does not use the maintainability index to avoid double-counting.",
                "range": "[0, 100]",
                "interpretation": "higher = more urgent to refactor"
            },
            "effort": {
                "name": "Effort Estimate",
                "description": "Heuristic effort estimate based on file size, function count, and fan-in. Low: <100 lines, \u{2264}3 functions, <5 importers. High: \u{2265}500 lines, \u{2265}20 importers, or \u{2265}15 functions with high density. Medium: everything else.",
                "values": ["low", "medium", "high"],
                "interpretation": "low = quick win, high = needs planning and coordination"
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── rule_by_id ───────────────────────────────────────────────────

    #[test]
    fn rule_by_id_finds_check_rule() {
        let rule = rule_by_id("fallow/unused-file").unwrap();
        assert_eq!(rule.name, "Unused Files");
    }

    #[test]
    fn rule_by_id_finds_health_rule() {
        let rule = rule_by_id("fallow/high-cyclomatic-complexity").unwrap();
        assert_eq!(rule.name, "High Cyclomatic Complexity");
    }

    #[test]
    fn rule_by_id_finds_dupes_rule() {
        let rule = rule_by_id("fallow/code-duplication").unwrap();
        assert_eq!(rule.name, "Code Duplication");
    }

    #[test]
    fn rule_by_id_returns_none_for_unknown() {
        assert!(rule_by_id("fallow/nonexistent").is_none());
        assert!(rule_by_id("").is_none());
    }

    // ── rule_docs_url ────────────────────────────────────────────────

    #[test]
    fn rule_docs_url_format() {
        let rule = rule_by_id("fallow/unused-export").unwrap();
        let url = rule_docs_url(rule);
        assert!(url.starts_with("https://docs.fallow.tools/"));
        assert!(url.contains("unused-exports"));
    }

    // ── CHECK_RULES completeness ─────────────────────────────────────

    #[test]
    fn check_rules_all_have_fallow_prefix() {
        for rule in CHECK_RULES {
            assert!(
                rule.id.starts_with("fallow/"),
                "rule {} should start with fallow/",
                rule.id
            );
        }
    }

    #[test]
    fn check_rules_all_have_docs_path() {
        for rule in CHECK_RULES {
            assert!(
                !rule.docs_path.is_empty(),
                "rule {} should have a docs_path",
                rule.id
            );
        }
    }

    #[test]
    fn check_rules_no_duplicate_ids() {
        let mut seen = std::collections::HashSet::new();
        for rule in CHECK_RULES.iter().chain(HEALTH_RULES).chain(DUPES_RULES) {
            assert!(seen.insert(rule.id), "duplicate rule id: {}", rule.id);
        }
    }

    // ── check_meta ───────────────────────────────────────────────────

    #[test]
    fn check_meta_has_docs_and_rules() {
        let meta = check_meta();
        assert!(meta.get("docs").is_some());
        assert!(meta.get("rules").is_some());
        let rules = meta["rules"].as_object().unwrap();
        // Verify all 13 rule categories are present (stripped fallow/ prefix)
        assert_eq!(rules.len(), CHECK_RULES.len());
        assert!(rules.contains_key("unused-file"));
        assert!(rules.contains_key("unused-export"));
        assert!(rules.contains_key("unused-type"));
        assert!(rules.contains_key("unused-dependency"));
        assert!(rules.contains_key("unused-dev-dependency"));
        assert!(rules.contains_key("unused-optional-dependency"));
        assert!(rules.contains_key("unused-enum-member"));
        assert!(rules.contains_key("unused-class-member"));
        assert!(rules.contains_key("unresolved-import"));
        assert!(rules.contains_key("unlisted-dependency"));
        assert!(rules.contains_key("duplicate-export"));
        assert!(rules.contains_key("type-only-dependency"));
        assert!(rules.contains_key("circular-dependency"));
    }

    #[test]
    fn check_meta_rule_has_required_fields() {
        let meta = check_meta();
        let rules = meta["rules"].as_object().unwrap();
        for (key, value) in rules {
            assert!(value.get("name").is_some(), "rule {key} missing 'name'");
            assert!(
                value.get("description").is_some(),
                "rule {key} missing 'description'"
            );
            assert!(value.get("docs").is_some(), "rule {key} missing 'docs'");
        }
    }

    // ── health_meta ──────────────────────────────────────────────────

    #[test]
    fn health_meta_has_metrics() {
        let meta = health_meta();
        assert!(meta.get("docs").is_some());
        let metrics = meta["metrics"].as_object().unwrap();
        assert!(metrics.contains_key("cyclomatic"));
        assert!(metrics.contains_key("cognitive"));
        assert!(metrics.contains_key("maintainability_index"));
        assert!(metrics.contains_key("complexity_density"));
        assert!(metrics.contains_key("fan_in"));
        assert!(metrics.contains_key("fan_out"));
    }

    // ── dupes_meta ───────────────────────────────────────────────────

    #[test]
    fn dupes_meta_has_metrics() {
        let meta = dupes_meta();
        assert!(meta.get("docs").is_some());
        let metrics = meta["metrics"].as_object().unwrap();
        assert!(metrics.contains_key("duplication_percentage"));
        assert!(metrics.contains_key("token_count"));
        assert!(metrics.contains_key("clone_groups"));
        assert!(metrics.contains_key("clone_families"));
    }
}

/// Build the `_meta` object for `fallow dupes --format json --explain`.
pub fn dupes_meta() -> Value {
    json!({
        "docs": DUPES_DOCS,
        "metrics": {
            "duplication_percentage": {
                "name": "Duplication Percentage",
                "description": "Fraction of total source tokens that appear in at least one clone group. Computed over the full analyzed file set.",
                "range": "[0, 100]",
                "interpretation": "lower is better"
            },
            "token_count": {
                "name": "Token Count",
                "description": "Number of normalized source tokens in the clone group. Tokens are language-aware (keywords, identifiers, operators, punctuation). Higher token count = larger duplicate.",
                "range": "[1, \u{221e})",
                "interpretation": "larger clones have higher refactoring value"
            },
            "line_count": {
                "name": "Line Count",
                "description": "Number of source lines spanned by the clone instance. Approximation of clone size for human readability.",
                "range": "[1, \u{221e})",
                "interpretation": "larger clones are more impactful to deduplicate"
            },
            "clone_groups": {
                "name": "Clone Groups",
                "description": "A set of code fragments with identical or near-identical normalized token sequences. Each group has 2+ instances across different locations.",
                "interpretation": "each group is a single refactoring opportunity"
            },
            "clone_families": {
                "name": "Clone Families",
                "description": "Groups of clone groups that share the same set of files. Indicates systematic duplication patterns (e.g., mirrored directory structures).",
                "interpretation": "families suggest extract-module refactoring opportunities"
            }
        }
    })
}
