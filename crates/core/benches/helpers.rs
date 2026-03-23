use std::path::PathBuf;

use fallow_config::{FallowConfig, OutputFormat};

pub fn create_test_config(root: PathBuf) -> fallow_config::ResolvedConfig {
    make_config(root, true)
}

pub fn make_config(root: PathBuf, no_cache: bool) -> fallow_config::ResolvedConfig {
    FallowConfig {
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
    .resolve(root, OutputFormat::Human, 4, no_cache, true)
}

/// Generate a synthetic project with `file_count` source files.
/// Half of the exports are consumed by the entry point, the other half are "dead".
pub fn create_synthetic_project(
    name: &str,
    file_count: usize,
) -> (PathBuf, fallow_config::ResolvedConfig) {
    create_synthetic_project_with_cache(name, file_count, true)
}

pub fn create_synthetic_project_with_cache(
    name: &str,
    file_count: usize,
    no_cache: bool,
) -> (PathBuf, fallow_config::ResolvedConfig) {
    let temp_dir = std::env::temp_dir().join(format!("fallow-bench-{name}"));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();

    std::fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "bench-project", "main": "src/index.ts", "dependencies": {"react": "^18"}}"#,
    )
    .unwrap();

    for i in 0..file_count {
        let content = format!(
            r#"
export const value{i} = {i};
export function fn{i}() {{ return {i}; }}
export type Type{i} = {{ value: number }};
export const helper{i} = () => value{i} + 1;
"#
        );
        std::fs::write(temp_dir.join(format!("src/module{i}.ts")), content).unwrap();
    }

    // Entry point imports from the first half of modules
    let used_count = file_count / 2;
    let imports: Vec<String> = (0..used_count)
        .map(|i| format!("import {{ value{i} }} from './module{i}';"))
        .collect();
    let uses: Vec<String> = (0..used_count)
        .map(|i| format!("console.log(value{i});"))
        .collect();
    std::fs::write(
        temp_dir.join("src/index.ts"),
        format!("{}\n{}\n", imports.join("\n"), uses.join("\n")),
    )
    .unwrap();

    let config = make_config(temp_dir.clone(), no_cache);
    (temp_dir, config)
}

/// Generate a synthetic project with duplicated code blocks for dupe detection benchmarks.
/// ~40% of files contain shared code blocks (each ~30 lines), rest is unique.
#[allow(dead_code)] // Used by large_analysis bench target
pub fn create_dupe_project(
    name: &str,
    file_count: usize,
) -> (PathBuf, fallow_config::ResolvedConfig) {
    let temp_dir = std::env::temp_dir().join(format!("fallow-bench-dupes-{name}"));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();

    std::fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "bench-dupes", "main": "src/index.ts"}"#,
    )
    .unwrap();

    // Generate shared duplicated code blocks (~30 lines each)
    let dupe_groups = file_count / 25;
    let blocks: Vec<String> = (0..dupe_groups)
        .map(|g| {
            let mut block = String::new();
            block.push_str(&format!(
                "export const processData_{g} = (input: string): Record<string, unknown> => {{\n"
            ));
            block.push_str("  const result: Record<string, unknown> = {};\n");
            block.push_str("  const timestamp = Date.now();\n");
            block.push_str(&format!("  const id = `item_${{timestamp}}_{g}`;\n"));
            block.push_str("  if (!input) {\n");
            block.push_str(&format!(
                "    throw new Error('Input is required for group {g}');\n"
            ));
            block.push_str("  }\n");
            block.push_str("  result.id = id;\n");
            block.push_str("  result.status = 'active';\n");
            block.push_str("  result.createdAt = new Date(timestamp).toISOString();\n");
            block.push_str("  result.updatedAt = new Date(timestamp).toISOString();\n");
            for line in 0..18 {
                block.push_str(&format!(
                    "  result.field_{line} = String(input).slice(0, {});\n",
                    10 + line * 3
                ));
            }
            block.push_str("  return result;\n};\n");
            block
        })
        .collect();

    // ~40% of files get at least one dupe block, each group appears in 2-3 files
    let dupe_file_count = file_count * 2 / 5;
    for i in 0..file_count {
        let mut content = String::new();
        // Unique content
        content.push_str(&format!(
            "export const unique_{i} = (v: string): string => `${{v}}_{i}`;\n\n"
        ));
        // Add dupe block if within dupe range
        if i < dupe_file_count && !blocks.is_empty() {
            let group = i % blocks.len();
            content.push_str(&blocks[group]);
            content.push('\n');
        }
        // More unique filler
        content.push_str(&format!("export const helper_{i} = {i};\n"));
        std::fs::write(temp_dir.join(format!("src/module{i}.ts")), content).unwrap();
    }

    std::fs::write(temp_dir.join("src/index.ts"), "export const main = true;\n").unwrap();

    let config = create_test_config(temp_dir.clone());
    (temp_dir, config)
}
