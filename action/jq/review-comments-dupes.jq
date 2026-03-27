def prefix: $ENV.PREFIX // "";
def root: $ENV.FALLOW_ROOT // ".";
def repo: $ENV.GH_REPO // "";
def pr: $ENV.PR_NUMBER // "";
def sha: $ENV.PR_HEAD_SHA // "";
def rel_path: if startswith("/") then (. as $p | root as $r | if ($p | test("/\($r)/")) then ($p | capture("/\($r)/(?<rest>.*)") | .rest) else ($p | split("/") | .[-3:] | join("/")) end) else . end;
def file_link(path; start; end_line):
  if (repo | length) > 0 and (sha | length) > 0 then
    "[`\(path):\(start)-\(end_line)`](https://github.com/\(repo)/blob/\(sha)/\(prefix)\(path)#L\(start)-L\(end_line))"
  else "`\(path):\(start)-\(end_line)`" end;
def footer: "\n\n---\n<sub><a href=\"https://docs.fallow.tools/explanations/duplication\">Docs</a> \u00b7 Disagree? <a href=\"https://docs.fallow.tools/configuration/suppression\">Configure or suppress</a></sub>";
[
  (.clone_families // [])[] | . as $family |
    ($family.suggestions // []) as $suggestions |
    $family.groups[]? | . as $group |
    ($group.instances | length) as $count |
    .instances[]? | . as $inst |
      ($inst.file | rel_path) as $this_path |
      ($group.instances | map(select(. != $inst)) |
        map((.file | rel_path) as $p | "- " + file_link($p; .start_line; .end_line)) | join("\n")) as $others |
      {
        path: (prefix + $this_path),
        start_line: $inst.start_line,
        line: $inst.end_line,
        body: ":warning: **Code duplication**\n\n**\($group.line_count) duplicated lines** \u00b7 \($group.token_count) tokens \u00b7 \($count) instances\n\nAlso found in:\n\($others)\n\n\(if $inst.fragment then "<details>\n<summary>View duplicated code</summary>\n\n```ts\n\($inst.fragment)\n```\n</details>\n\n" else "" end)\(if ($suggestions | length) > 0 then ($suggestions | map(":bulb: **Suggestion:** \(.description)\(if .estimated_savings then " (~\(.estimated_savings) lines saved)" else "" end)") | join("\n")) + "\n" else "**Action:** Extract a shared function to keep both code paths in sync and eliminate duplication.\n" end)\(footer)"
      }
] | .[:($ENV.MAX | tonumber)]
