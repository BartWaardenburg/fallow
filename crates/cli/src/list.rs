use std::process::ExitCode;

use fallow_config::OutputFormat;

use crate::load_config;

#[allow(clippy::struct_excessive_bools)]
pub struct ListOptions<'a> {
    pub root: &'a std::path::Path,
    pub config_path: &'a Option<std::path::PathBuf>,
    pub output: OutputFormat,
    pub threads: usize,
    pub entry_points: bool,
    pub files: bool,
    pub plugins: bool,
    pub production: bool,
}

pub fn run_list(opts: &ListOptions<'_>) -> ExitCode {
    let config = match load_config(
        opts.root,
        opts.config_path,
        OutputFormat::Human,
        true,
        opts.threads,
        opts.production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let show_all = !opts.entry_points && !opts.files && !opts.plugins;

    // Run plugin detection to find active plugins (including workspace packages)
    let plugin_result = if opts.plugins || show_all {
        let disc = fallow_core::discover::discover_files(&config);
        let file_paths: Vec<std::path::PathBuf> = disc.iter().map(|f| f.path.clone()).collect();
        let registry = fallow_core::plugins::PluginRegistry::new(config.external_plugins.clone());

        let pkg_path = opts.root.join("package.json");
        let mut result = fallow_config::PackageJson::load(&pkg_path).map_or_else(
            |_| fallow_core::plugins::AggregatedPluginResult::default(),
            |pkg| registry.run(&pkg, opts.root, &file_paths),
        );

        // Also run plugins for workspace packages
        let workspaces = fallow_config::discover_workspaces(opts.root);
        for ws in &workspaces {
            let ws_pkg_path = ws.root.join("package.json");
            if let Ok(ws_pkg) = fallow_config::PackageJson::load(&ws_pkg_path) {
                let ws_result = registry.run(&ws_pkg, &ws.root, &file_paths);
                for plugin_name in &ws_result.active_plugins {
                    if !result.active_plugins.contains(plugin_name) {
                        result.active_plugins.push(plugin_name.clone());
                    }
                }
            }
        }
        Some(result)
    } else {
        None
    };

    // Discover files once if needed by either files or entry_points
    let need_files = opts.files || show_all || opts.entry_points;
    let discovered = if need_files {
        Some(fallow_core::discover::discover_files(&config))
    } else {
        None
    };

    // Compute entry points once (shared by both JSON and human output branches)
    let all_entry_points = if (opts.entry_points || show_all)
        && let Some(ref disc) = discovered
    {
        let mut entries = fallow_core::discover::discover_entry_points(&config, disc);
        // Add workspace entry points
        let workspaces = fallow_config::discover_workspaces(opts.root);
        for ws in &workspaces {
            let ws_entries =
                fallow_core::discover::discover_workspace_entry_points(&ws.root, &config, disc);
            entries.extend(ws_entries);
        }
        // Add plugin-discovered entry points
        if let Some(ref pr) = plugin_result {
            let plugin_entries =
                fallow_core::discover::discover_plugin_entry_points(pr, &config, disc);
            entries.extend(plugin_entries);
        }
        Some(entries)
    } else {
        None
    };

    match opts.output {
        OutputFormat::Json => {
            let mut result = serde_json::Map::new();

            if (opts.plugins || show_all)
                && let Some(ref pr) = plugin_result
            {
                let pl: Vec<serde_json::Value> = pr
                    .active_plugins
                    .iter()
                    .map(|name| serde_json::json!({ "name": name }))
                    .collect();
                result.insert("plugins".to_string(), serde_json::json!(pl));
            }

            if (opts.files || show_all)
                && let Some(ref disc) = discovered
            {
                let paths: Vec<serde_json::Value> = disc
                    .iter()
                    .map(|f| {
                        let relative = f.path.strip_prefix(opts.root).unwrap_or(&f.path);
                        serde_json::json!(relative.display().to_string())
                    })
                    .collect();
                result.insert("file_count".to_string(), serde_json::json!(paths.len()));
                result.insert("files".to_string(), serde_json::json!(paths));
            }

            if let Some(ref entries) = all_entry_points {
                let eps: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|ep| {
                        let relative = ep.path.strip_prefix(opts.root).unwrap_or(&ep.path);
                        serde_json::json!({
                            "path": relative.display().to_string(),
                            "source": format!("{:?}", ep.source),
                        })
                    })
                    .collect();
                result.insert(
                    "entry_point_count".to_string(),
                    serde_json::json!(eps.len()),
                );
                result.insert("entry_points".to_string(), serde_json::json!(eps));
            }

            match serde_json::to_string_pretty(&serde_json::Value::Object(result)) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("Error: failed to serialize list output: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        _ => {
            if (opts.plugins || show_all)
                && let Some(ref pr) = plugin_result
            {
                eprintln!("Active plugins:");
                for name in &pr.active_plugins {
                    eprintln!("  - {name}");
                }
            }

            if (opts.files || show_all)
                && let Some(ref disc) = discovered
            {
                eprintln!("Discovered {} files", disc.len());
                for file in disc {
                    println!("{}", file.path.display());
                }
            }

            if let Some(ref entries) = all_entry_points {
                eprintln!("Found {} entry points", entries.len());
                for ep in entries {
                    println!("{} ({:?})", ep.path.display(), ep.source);
                }
            }
        }
    }

    ExitCode::SUCCESS
}
