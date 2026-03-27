def prefix: $ENV.PREFIX // "";
def root: $ENV.FALLOW_ROOT // ".";
def rel_path: if startswith("/") then (. as $p | root as $r | if ($p | test("/\($r)/")) then ($p | capture("/\($r)/(?<rest>.*)") | .rest) else ($p | split("/") | .[-3:] | join("/")) end) else . end;
def footer: "\n\n---\n<sub>\ud83c\udf3f <a href=\"https://docs.fallow.tools/explanations/health\">complexity</a> \u00b7 <a href=\"https://docs.fallow.tools/cli/health\">CLI reference</a> \u00b7 Configure thresholds in <code>.fallowrc.json</code></sub>";
(.summary.max_cyclomatic_threshold // 20) as $cyc_t |
(.summary.max_cognitive_threshold // 15) as $cog_t |
[
  (.findings[]? | {
    path: (prefix + (.path | rel_path)),
    line: .line,
    body: ":warning: **High complexity**\n\nFunction `\(.name)` exceeds complexity thresholds:\n\n| Metric | Value | Threshold | Status |\n|:-------|------:|----------:|:------:|\n| [Cyclomatic](https://docs.fallow.tools/explanations/health#cyclomatic-complexity) | **\(.cyclomatic)** | \($cyc_t) | \(if .exceeded == "cyclomatic" or .exceeded == "both" then ":red_circle:" else ":white_check_mark:" end) |\n| [Cognitive](https://docs.fallow.tools/explanations/health#cognitive-complexity) | **\(.cognitive)** | \($cog_t) | \(if .exceeded == "cognitive" or .exceeded == "both" then ":red_circle:" else ":white_check_mark:" end) |\n| Lines | \(.line_count) | \u2014 | \u2014 |\n\n<details>\n<summary>What these metrics mean</summary>\n\n- **[Cyclomatic complexity](https://docs.fallow.tools/explanations/health#cyclomatic-complexity)** \u2014 McCabe complexity: 1 + decision points (if/else, switch, loops, ternary, logical operators). Counts independent code paths.\n- **[Cognitive complexity](https://docs.fallow.tools/explanations/health#cognitive-complexity)** \u2014 SonarSource model: penalizes nesting depth and non-linear control flow. Measures how hard a function is to read top-to-bottom.\n</details>\n\n**Action:** Split into smaller, focused functions. Consider extracting each `case` branch into a named helper.\(footer)"
  }),
  ((.targets // .refactoring_targets // [])[:5][]? |
    # Resolve line: use evidence.complex_functions[0].line or evidence.unused_exports line, fallback to 1
    (if .evidence.complex_functions then .evidence.complex_functions[0].line
     elif .evidence.unused_exports then 1
     else 1 end) as $target_line |
    {
    path: (prefix + (.path | rel_path)),
    line: $target_line,
    body: ":bulb: **Refactoring target**\n\n| Priority | Effort | Confidence |\n|:---------|:-------|:-----------|\n| \(.priority) | \(.effort) | \(.confidence) |\n\n\(.recommendation)\n\n\(if .factors then "**Contributing factors:**\n\(.factors | map("- [`\(.metric)`](https://docs.fallow.tools/explanations/health#\(.metric | gsub("_"; "-"))): \(.detail // (.value | tostring))") | join("\n"))\n" else "" end)\(if .evidence.complex_functions then "\n<details>\n<summary>Complex functions</summary>\n\n" + (.evidence.complex_functions | map("- `\(.name)` \u2014 cognitive: \(.cognitive), line \(.line)") | join("\n")) + "\n</details>\n" elif .evidence.unused_exports then "\n<details>\n<summary>Unused exports</summary>\n\n" + (.evidence.unused_exports | map("- `\(.)`") | join("\n")) + "\n</details>\n" else "" end)\(footer)"
  })
] | .[:($ENV.MAX | tonumber)]
