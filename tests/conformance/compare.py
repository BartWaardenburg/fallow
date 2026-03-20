#!/usr/bin/env python3
"""Compare fallow and knip JSON outputs and produce a conformance report.

Usage:
    python3 compare.py <fallow.json> <knip.json> <project_root>

Reads the JSON output from both tools, normalizes them into comparable
(file, export_name, issue_type) tuples, and reports agreement/disagreement.

Outputs a structured JSON report to stdout.
"""

import json
import os
import sys


# Issue type mapping: fallow key -> knip key
FALLOW_TO_KNIP = {
    "unused_files": "files",
    "unused_exports": "exports",
    "unused_types": "types",
    "unused_dependencies": "dependencies",
    "unused_dev_dependencies": "devDependencies",
    "unresolved_imports": "unresolved",
    "unlisted_dependencies": "unlisted",
    "duplicate_exports": "duplicates",
    "unused_enum_members": "enumMembers",
    "unused_class_members": "classMembers",
}

KNIP_TO_FALLOW = {v: k for k, v in FALLOW_TO_KNIP.items()}


def normalize_path(path, project_root):
    """Normalize a file path to be relative to the project root."""
    # Handle both absolute and relative paths
    if os.path.isabs(path):
        try:
            path = os.path.relpath(path, project_root)
        except ValueError:
            pass
    # Normalize separators to forward slashes for cross-platform consistency
    path = path.replace(os.sep, "/")
    # Strip leading ./ if present
    if path.startswith("./"):
        path = path[2:]
    return path


def extract_fallow_issues(data, project_root):
    """Extract normalized (file, name, issue_type) tuples from fallow JSON."""
    issues = set()

    # unused_files: [{path: "..."}]
    for item in data.get("unused_files", []):
        path = normalize_path(item["path"], project_root)
        issues.add((path, "", "unused_files"))

    # unused_exports: [{path: "...", export_name: "..."}]
    for item in data.get("unused_exports", []):
        path = normalize_path(item["path"], project_root)
        issues.add((path, item["export_name"], "unused_exports"))

    # unused_types: [{path: "...", export_name: "..."}]
    for item in data.get("unused_types", []):
        path = normalize_path(item["path"], project_root)
        issues.add((path, item["export_name"], "unused_types"))

    # unused_dependencies: [{package_name: "..."}]
    for item in data.get("unused_dependencies", []):
        pkg = item["package_name"]
        issues.add(("package.json", pkg, "unused_dependencies"))

    # unused_dev_dependencies: [{package_name: "..."}]
    for item in data.get("unused_dev_dependencies", []):
        pkg = item["package_name"]
        issues.add(("package.json", pkg, "unused_dev_dependencies"))

    # unresolved_imports: [{path: "...", specifier: "..."}]
    for item in data.get("unresolved_imports", []):
        path = normalize_path(item["path"], project_root)
        issues.add((path, item["specifier"], "unresolved_imports"))

    # unlisted_dependencies: [{package_name: "..."}]
    for item in data.get("unlisted_dependencies", []):
        pkg = item["package_name"]
        issues.add(("package.json", pkg, "unlisted_dependencies"))

    # duplicate_exports: [{export_name: "...", locations: [...]}]
    for item in data.get("duplicate_exports", []):
        name = item["export_name"]
        for loc in item["locations"]:
            path = normalize_path(loc, project_root)
            issues.add((path, name, "duplicate_exports"))

    # unused_enum_members: [{path, parent_name, member_name}]
    for item in data.get("unused_enum_members", []):
        path = normalize_path(item["path"], project_root)
        name = f"{item['parent_name']}.{item['member_name']}"
        issues.add((path, name, "unused_enum_members"))

    # unused_class_members: [{path, parent_name, member_name}]
    for item in data.get("unused_class_members", []):
        path = normalize_path(item["path"], project_root)
        name = f"{item['parent_name']}.{item['member_name']}"
        issues.add((path, name, "unused_class_members"))

    return issues


def extract_knip_issues(data, project_root):
    """Extract normalized (file, name, issue_type) tuples from knip JSON.

    Knip JSON format (v5+):
    {
      "files": ["path1", "path2"],
      "issues": [
        {
          "file": "path",
          "dependencies": [{"name": "...", "line": N, "col": N, "pos": N}],
          "devDependencies": [...],
          "exports": [{"name": "...", "line": N, "col": N, "pos": N}],
          "types": [...],
          "unresolved": [...],
          "unlisted": [...],
          "duplicates": [...],
          "enumMembers": {"EnumName": [{"name": "...", ...}]},
          "classMembers": {"ClassName": [{"name": "...", ...}]},
          ...
        }
      ]
    }
    """
    issues = set()

    # Unused files are listed at the top level
    for filepath in data.get("files", []):
        path = normalize_path(filepath, project_root)
        issues.add((path, "", "unused_files"))

    # Per-file issues
    for file_entry in data.get("issues", []):
        filepath = file_entry.get("file", "")
        path = normalize_path(filepath, project_root)

        # Named issue types where values are [{name, line, col, pos}]
        named_types = {
            "dependencies": "unused_dependencies",
            "devDependencies": "unused_dev_dependencies",
            "exports": "unused_exports",
            "types": "unused_types",
            "unresolved": "unresolved_imports",
            "unlisted": "unlisted_dependencies",
            "duplicates": "duplicate_exports",
        }

        for knip_key, fallow_type in named_types.items():
            for item in file_entry.get(knip_key, []):
                name = item.get("name", "") if isinstance(item, dict) else str(item)
                # For dependency types, use package.json as the file
                if fallow_type in (
                    "unused_dependencies",
                    "unused_dev_dependencies",
                    "unlisted_dependencies",
                ):
                    issues.add(("package.json", name, fallow_type))
                else:
                    issues.add((path, name, fallow_type))

        # Enum members: {"EnumName": [{"name": "MemberName", ...}]}
        enum_members = file_entry.get("enumMembers", {})
        if isinstance(enum_members, dict):
            for parent_name, members in enum_members.items():
                if isinstance(members, list):
                    for member in members:
                        member_name = (
                            member.get("name", "")
                            if isinstance(member, dict)
                            else str(member)
                        )
                        issues.add(
                            (path, f"{parent_name}.{member_name}", "unused_enum_members")
                        )

        # Class members: {"ClassName": [{"name": "MemberName", ...}]}
        class_members = file_entry.get("classMembers", {})
        if isinstance(class_members, dict):
            for parent_name, members in class_members.items():
                if isinstance(members, list):
                    for member in members:
                        member_name = (
                            member.get("name", "")
                            if isinstance(member, dict)
                            else str(member)
                        )
                        issues.add(
                            (
                                path,
                                f"{parent_name}.{member_name}",
                                "unused_class_members",
                            )
                        )

    return issues


def compare_issues(fallow_issues, knip_issues):
    """Compare two sets of issues and produce a comparison report."""
    agreed = fallow_issues & knip_issues
    fallow_only = fallow_issues - knip_issues
    knip_only = knip_issues - fallow_issues
    total_unique = len(fallow_issues | knip_issues)

    agreement_pct = (len(agreed) / total_unique * 100) if total_unique > 0 else 100.0

    return {
        "agreed": sorted(agreed),
        "fallow_only": sorted(fallow_only),
        "knip_only": sorted(knip_only),
        "total_unique": total_unique,
        "agreement_pct": round(agreement_pct, 1),
    }


def build_report(fallow_issues, knip_issues, comparison):
    """Build a structured JSON report from the comparison."""
    # Break down by issue type
    all_types = set()
    for _, _, issue_type in fallow_issues | knip_issues:
        all_types.add(issue_type)

    by_type = {}
    for issue_type in sorted(all_types):
        f_typed = {(f, n, t) for f, n, t in fallow_issues if t == issue_type}
        k_typed = {(f, n, t) for f, n, t in knip_issues if t == issue_type}
        agreed = f_typed & k_typed
        f_only = f_typed - k_typed
        k_only = k_typed - f_typed
        total = len(f_typed | k_typed)
        pct = (len(agreed) / total * 100) if total > 0 else 100.0

        by_type[issue_type] = {
            "fallow_count": len(f_typed),
            "knip_count": len(k_typed),
            "agreed": len(agreed),
            "fallow_only": len(f_only),
            "knip_only": len(k_only),
            "agreement_pct": round(pct, 1),
        }

    def issue_to_dict(issue):
        return {"file": issue[0], "name": issue[1], "type": issue[2]}

    return {
        "summary": {
            "fallow_total": len(fallow_issues),
            "knip_total": len(knip_issues),
            "agreed": len(comparison["agreed"]),
            "fallow_only": len(comparison["fallow_only"]),
            "knip_only": len(comparison["knip_only"]),
            "agreement_pct": comparison["agreement_pct"],
        },
        "by_type": by_type,
        "details": {
            "agreed": [issue_to_dict(i) for i in comparison["agreed"]],
            "fallow_only": [issue_to_dict(i) for i in comparison["fallow_only"]],
            "knip_only": [issue_to_dict(i) for i in comparison["knip_only"]],
        },
    }


def main():
    if len(sys.argv) != 4:
        print(
            f"Usage: {sys.argv[0]} <fallow.json> <knip.json> <project_root>",
            file=sys.stderr,
        )
        sys.exit(1)

    fallow_path = sys.argv[1]
    knip_path = sys.argv[2]
    project_root = os.path.abspath(sys.argv[3])

    with open(fallow_path) as f:
        fallow_data = json.load(f)

    with open(knip_path) as f:
        knip_data = json.load(f)

    fallow_issues = extract_fallow_issues(fallow_data, project_root)
    knip_issues = extract_knip_issues(knip_data, project_root)
    comparison = compare_issues(fallow_issues, knip_issues)
    report = build_report(fallow_issues, knip_issues, comparison)

    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
