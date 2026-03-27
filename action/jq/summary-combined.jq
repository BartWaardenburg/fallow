def count(obj; key): obj | if . then .[key] // 0 else 0 end;
def pct(n): n | . * 10 | round / 10;
def rel_path: split("/") | if length > 3 then .[-3:] | join("/") else join("/") end;

(count(.check; "total_issues")) as $check |
(count(.dupes.stats; "clone_groups")) as $dupes |
(count(.health.summary; "functions_above_threshold")) as $health |
($check + $dupes + $health) as $total |

# Vital signs
(.health.vital_signs // {}) as $vitals |
(.health.summary // {}) as $summary |
(.dupes.stats // {}) as $dupes_stats |

if $total == 0 then
  "# \ud83c\udf3f Fallow \u2014 Codebase Analysis\n\n" +
  "> [!NOTE]\n> **No issues found**\n\n" +
  "| Metric | Status |\n|:-------|:-------|\n" +
  "| Dead code | :white_check_mark: Clean |\n" +
  "| Duplication | :white_check_mark: Clean |\n" +
  "| Complexity | :white_check_mark: Clean |\n" +
  (if $vitals.maintainability_avg then "\n**Maintainability index:** \(pct($vitals.maintainability_avg))/100" else "" end)
else
  "# \ud83c\udf3f Fallow \u2014 Codebase Analysis\n\n" +
  "> [!WARNING]\n> **\($total) issues** found\n\n" +

  # Overview table
  "| Analysis | Status | Details |\n|:---------|:-------|:--------|\n" +
  (if $check > 0 then
    "| :warning: Dead code | **\($check) issues** | " +
    ([
      (if (.check.unused_exports | length) > 0 then "\((.check.unused_exports | length)) unused exports" else null end),
      (if (.check.unused_files | length) > 0 then "\((.check.unused_files | length)) unused files" else null end),
      (if (.check.unused_dependencies | length) > 0 then "\((.check.unused_dependencies | length)) unused deps" else null end),
      (if (.check.unresolved_imports | length) > 0 then "\((.check.unresolved_imports | length)) unresolved imports" else null end),
      (if (.check.circular_dependencies | length) > 0 then "\((.check.circular_dependencies | length)) circular deps" else null end)
    ] | map(select(. != null)) | join(", ")) +
    " |\n"
  else "| :white_check_mark: Dead code | Clean | \u2014 |\n" end) +
  (if $dupes > 0 then
    "| :warning: Duplication | **\($dupes) clone groups** | \($dupes_stats.duplicated_lines) duplicated lines (\(pct($dupes_stats.duplication_percentage))%) |\n"
  else "| :white_check_mark: Duplication | Clean | \u2014 |\n" end) +
  (if $health > 0 then
    "| :warning: Complexity | **\($health) functions** | above threshold (\($summary.functions_analyzed) analyzed) |\n"
  else "| :white_check_mark: Complexity | Clean | \($summary.functions_analyzed // 0) functions analyzed |\n" end) +

  # Vital signs
  (if $vitals | length > 0 then
    "\n### Vital signs\n\n" +
    "| Metric | Value |\n|:-------|------:|\n" +
    (if $vitals.maintainability_avg then "| Maintainability index | **\(pct($vitals.maintainability_avg))** / 100 |\n" else "" end) +
    (if $vitals.dead_export_pct then "| Dead exports | \(pct($vitals.dead_export_pct))% |\n" else "" end) +
    (if $vitals.avg_cyclomatic then "| Avg cyclomatic complexity | \(pct($vitals.avg_cyclomatic)) |\n" else "" end) +
    (if $vitals.p90_cyclomatic then "| P90 cyclomatic complexity | \($vitals.p90_cyclomatic) |\n" else "" end) +
    (if $vitals.unused_dep_count then "| Unused dependencies | \($vitals.unused_dep_count) |\n" else "" end)
  else "" end) +

  # Top findings
  (if (.health.findings // [] | length) > 0 then
    "\n<details>\n<summary><strong>Complexity hotspots (\(.health.findings | length))</strong></summary>\n\n" +
    "| File | Function | Cyclomatic | Cognitive |\n|:-----|:---------|----------:|---------:|\n" +
    ([.health.findings[:5][] |
      "| `\(.path | rel_path):\(.line)` | `\(.name)` | \(.cyclomatic) | \(.cognitive) |"
    ] | join("\n")) +
    "\n\n</details>\n"
  else "" end) +

  # Refactoring targets
  (if (.health.targets // [] | length) > 0 then
    "\n<details>\n<summary><strong>Refactoring targets (\(.health.targets | length))</strong></summary>\n\n" +
    ([.health.targets[:3][] |
      "- **\(.path | rel_path)** \u2014 \(.recommendation) *(effort: \(.effort), confidence: \(.confidence))*"
    ] | join("\n")) +
    "\n\n</details>\n"
  else "" end) +

  # Fix suggestion
  "\n> [!TIP]\n> Run `fallow fix --dry-run` to preview auto-fixes, or see the inline review comments for per-finding details.\n> Add `// fallow-ignore-next-line` above a line to suppress a specific finding."
end
