# Post-processing: merge, deduplicate, and group review comments
# Input: array of {path, line, body, ?type, ?export_name, ?group_id}
# Output: array of {path, line, body} — merged and deduplicated

# Step 1: Group unused exports per file into single comments
def group_unused_exports:
  . as $all |
  ($all | map(select(.type == "unused-export")) | group_by(.path)) as $groups |
  ($all | map(select(.type != "unused-export"))) +
  [$groups[] |
    if length == 1 then .[0]
    elif length > 1 then
      {
        path: .[0].path,
        line: .[0].line,
        type: "unused-export-group",
        body: (
          ":warning: **\(length) unused exports in this file**\n\n" +
          "The following exports are never imported by other modules:\n\n" +
          (map("- `\(.export_name)` *(line \(.line))*") | join("\n")) +
          "\n\n<details>\n<summary>Why this matters</summary>\n\nUnused exports signal to other developers that this code is used elsewhere \u2014 so nobody touches it, even when it should change. They also prevent bundlers from tree-shaking this code out of production.\n</details>\n\n**Action:** Remove the `export` keyword from each, or delete the declarations entirely.\n\n> Intentionally public? Add a `/** @public */` JSDoc tag above exports that are part of your API.\n\n---\n<sub><a href=\"https://docs.fallow.tools/explanations/dead-code#unused-exports\">Docs</a> \u00b7 Disagree? <a href=\"https://docs.fallow.tools/configuration/suppression\">Configure or suppress</a></sub>"
        )
      }
    else empty end
  ];

# Step 2: Keep only first instance per clone group
def dedup_clones:
  . as $all |
  ($all | map(select(.type == "duplication")) | group_by(.group_id) | map(.[0])) as $first_clones |
  ($all | map(select(.type != "duplication"))) + $first_clones;

# Step 3: Drop refactoring targets (they duplicate existing findings)
def drop_targets:
  map(select(.type != "refactoring-target"));

# Step 4: Merge comments on the same file+line
def merge_same_line:
  group_by("\(.path):\(.line)") |
  [.[] |
    if length == 1 then .[0]
    else
      {
        path: .[0].path,
        line: .[0].line,
        body: ([.[].body] | join("\n\n---\n\n"))
      }
    end
  ];

# Pipeline
group_unused_exports | dedup_clones | drop_targets | merge_same_line | .[:($ENV.MAX | tonumber)]
