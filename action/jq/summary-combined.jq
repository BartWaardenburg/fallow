def count(obj; key): obj | if . then .[key] // 0 else 0 end;
def pct(n): n | . * 10 | round / 10;
def rel_path: split("/") | if length > 3 then .[-3:] | join("/") else join("/") end;

(count(.check; "total_issues")) as $check |
(count(.dupes.stats; "clone_groups")) as $dupes |
(count(.health.summary; "functions_above_threshold")) as $health |
($check + $dupes + $health) as $total |
(.health.vital_signs // {}) as $vitals |
(.health.summary // {}) as $summary |
(.dupes.stats // {}) as $dupes_stats |

if $total == 0 then
  "# \ud83c\udf3f Fallow\n\n" +
  "> [!NOTE]\n> **No issues found**\n\n" +
  ":white_check_mark: Dead code \u00b7 :white_check_mark: Duplication \u00b7 :white_check_mark: Complexity" +
  (if $vitals.maintainability_avg then "\n\nMaintainability: **\(pct($vitals.maintainability_avg))** / 100" else "" end)
else
  "# \ud83c\udf3f Fallow\n\n" +

  # One-line status
  (if $check > 0 then ":warning: **\($check)** dead code" else ":white_check_mark: Dead code" end) +
  " \u00b7 " +
  (if $dupes > 0 then ":warning: **\($dupes)** clone groups" else ":white_check_mark: Duplication" end) +
  " \u00b7 " +
  (if $health > 0 then ":warning: **\($health)** complex functions" else ":white_check_mark: Complexity" end) +
  "\n\n" +

  # Dead code breakdown
  (if $check > 0 then
    "<details>\n<summary><strong>Dead code (\($check) issues)</strong></summary>\n\n" +
    "| Category | Count |\n|:---------|------:|\n" +
    ([
      (if (.check.unused_files | length) > 0 then "| Unused files | \(.check.unused_files | length) |" else null end),
      (if (.check.unused_exports | length) > 0 then "| Unused exports | \(.check.unused_exports | length) |" else null end),
      (if (.check.unused_dependencies | length) > 0 then "| Unused dependencies | \(.check.unused_dependencies | length) |" else null end),
      (if (.check.unresolved_imports | length) > 0 then "| Unresolved imports | \(.check.unresolved_imports | length) |" else null end),
      (if (.check.circular_dependencies | length) > 0 then "| Circular dependencies | \(.check.circular_dependencies | length) |" else null end),
      (if (.check.type_only_dependencies | length) > 0 then "| Type-only dependencies | \(.check.type_only_dependencies | length) |" else null end)
    ] | map(select(. != null)) | join("\n")) +
    "\n\n</details>\n\n"
  else "" end) +

  # Duplication breakdown
  (if $dupes > 0 then
    "<details>\n<summary><strong>Duplication (\($dupes) clone groups, \(pct($dupes_stats.duplication_percentage))%)</strong></summary>\n\n" +
    "| Metric | Value |\n|:-------|------:|\n" +
    "| Duplicated lines | \($dupes_stats.duplicated_lines) |\n" +
    "| Clone instances | \($dupes_stats.clone_instances) |\n" +
    "| Files with clones | \($dupes_stats.files_with_clones) |\n" +
    "\n</details>\n\n"
  else "" end) +

  # Complexity breakdown
  (if $health > 0 then
    "<details>\n<summary><strong>Complexity (\($health) functions above threshold)</strong></summary>\n\n" +
    "| File | Function | Cyclomatic | Cognitive |\n|:-----|:---------|----------:|---------:|\n" +
    ([.health.findings[:5][] |
      "| `\(.path | rel_path):\(.line)` | `\(.name)` | \(.cyclomatic) | \(.cognitive) |"
    ] | join("\n")) +
    "\n\n</details>\n\n"
  else "" end) +

  # Vital signs
  (if $vitals | length > 0 then
    "| Metric | Value |\n|:-------|------:|\n" +
    (if $vitals.maintainability_avg then "| Maintainability | **\(pct($vitals.maintainability_avg))** / 100 |\n" else "" end) +
    (if $vitals.dead_export_pct then "| Dead exports | \(pct($vitals.dead_export_pct))% |\n" else "" end) +
    (if $vitals.avg_cyclomatic then "| Avg complexity | \(pct($vitals.avg_cyclomatic)) |\n" else "" end) +
    "\n"
  else "" end) +

  "> [!TIP]\n> Run `fallow fix --dry-run` to preview auto-fixes. Add `/** @public */` above exports to preserve them.\n> See inline review comments for per-finding details."
end
