def prefix: $ENV.PREFIX // "";
def root: $ENV.FALLOW_ROOT // ".";
def rel_path: if startswith("/") then (. as $p | root as $r | if ($p | test("/\($r)/")) then ($p | capture("/\($r)/(?<rest>.*)") | .rest) else ($p | split("/") | .[-3:] | join("/")) end) else . end;
def footer: "\n\n---\n<sub><a href=\"https://docs.fallow.tools/explanations/health\">Docs</a> \u00b7 Disagree? <a href=\"https://docs.fallow.tools/configuration/rules\">Configure thresholds</a></sub>";
(.summary.max_cyclomatic_threshold // 20) as $cyc_t |
(.summary.max_cognitive_threshold // 15) as $cog_t |
[
  (.findings[]? | {
    type: "other",
    path: (prefix + (.path | rel_path)),
    line: .line,
    body: ":warning: **High complexity**\n\nFunction `\(.name)` exceeds complexity thresholds:\n\n| Metric | Value | Threshold | Status |\n|:-------|------:|----------:|:------:|\n| [Cyclomatic](https://docs.fallow.tools/explanations/health#cyclomatic-complexity) | **\(.cyclomatic)** | \($cyc_t) | \(if .exceeded == "cyclomatic" or .exceeded == "both" then ":red_circle:" else ":white_check_mark:" end) |\n| [Cognitive](https://docs.fallow.tools/explanations/health#cognitive-complexity) | **\(.cognitive)** | \($cog_t) | \(if .exceeded == "cognitive" or .exceeded == "both" then ":red_circle:" else ":white_check_mark:" end) |\n| Lines | \(.line_count) | \u2014 | \u2014 |\n\n<details>\n<summary>What these metrics mean</summary>\n\n- **Cyclomatic complexity** \u2014 How many independent paths through this function (each `if`, `switch` case, loop, and `&&`/`||` adds one). High values mean more branches to test.\n- **Cognitive complexity** \u2014 How hard this function is to read top-to-bottom. Penalizes deeply nested logic and jumps in control flow.\n</details>\n\n**Action:** Break this into smaller functions, each doing one thing. Look for independent blocks of logic that can be extracted with a descriptive name.\(footer)"
  }),
  ((.targets // .refactoring_targets // [])[:5][]? |
    (if .evidence.complex_functions then .evidence.complex_functions[0].line
     else 1 end) as $target_line |
    {
    type: "refactoring-target",
    path: (prefix + (.path | rel_path)),
    line: $target_line,
    body: ":bulb: **Refactoring target**\n\n`\(.recommendation)`\n\n| Effort | Confidence |\n|:-------|:-----------|\n| \(.effort) | \(.confidence) |\n\n\(if .factors then "**Why:**\n\(.factors | map("- \(.detail // "\(.metric): \(.value)")") | join("\n"))\n" else "" end)\(if .evidence.complex_functions then "\n<details>\n<summary>Complex functions</summary>\n\n\(.evidence.complex_functions | map("- `\(.name)` \u2014 cognitive: \(.cognitive), line \(.line)") | join("\n"))\n</details>\n" elif .evidence.unused_exports then "\n<details>\n<summary>Unused exports</summary>\n\n\(.evidence.unused_exports | map("- `\(.)`") | join("\n"))\n</details>\n" else "" end)\(footer)"
  })
] | .[:($ENV.MAX | tonumber)]
