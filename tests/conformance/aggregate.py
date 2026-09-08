#!/usr/bin/env python3
"""Aggregate per-project conformance reports into a combined report.

Usage:
    python3 aggregate.py <reports_dir> [--adjudications PATH]

Reads all *-report.json files from the directory, combines them into a single
report with per-project breakdowns and overall summary.

Every disagreement row is joined against the adjudication register
(adjudications.json next to this script) so the report separates reviewed
disagreements from the backlog that nobody has looked at yet.

Outputs aggregated JSON to stdout.
"""

import json
import sys
from pathlib import Path

DEFAULT_REGISTER = Path(__file__).resolve().parent / "adjudications.json"

# Counter names emitted next to agreement_pct. Ordered so the three reviewed
# buckets read before the backlog.
ADJUDICATION_COUNTERS = (
    "adjudicated_fallow_wrong",
    "adjudicated_knip_wrong",
    "adjudicated_model_difference",
    "unadjudicated",
)


def load_register(path):
    """Load and validate the adjudication register.

    Returns a dict keyed on (project, issue_type, path, export_name) mapping to
    the record's verdict. A malformed register is a hard error: a register that
    silently drops records would understate the unadjudicated backlog.
    """
    if not path.exists():
        print(f"Error: adjudication register not found: {path}", file=sys.stderr)
        sys.exit(1)

    with open(path) as f:
        register = json.load(f)

    verdicts = set(register.get("verdicts", {}))
    causes = set(register.get("causes", {}))
    required = register.get("record_fields", [])

    index = {}
    for i, record in enumerate(register.get("adjudications", [])):
        missing = [field for field in required if field not in record]
        if missing:
            print(
                f"Error: adjudication {i} is missing {', '.join(missing)}",
                file=sys.stderr,
            )
            sys.exit(1)
        if record["verdict"] not in verdicts:
            print(
                f"Error: adjudication {i} has unknown verdict {record['verdict']!r}",
                file=sys.stderr,
            )
            sys.exit(1)
        if record["cause"] not in causes:
            print(
                f"Error: adjudication {i} has unknown cause {record['cause']!r}",
                file=sys.stderr,
            )
            sys.exit(1)

        key = (
            record["project"],
            record["issue_type"],
            record["path"],
            record["export_name"],
        )
        if key in index:
            print(f"Error: duplicate adjudication for {key}", file=sys.stderr)
            sys.exit(1)
        index[key] = record["verdict"]

    return index


def adjudicate(project, report, index):
    """Tally one project's disagreement rows against the register."""
    counts = dict.fromkeys(ADJUDICATION_COUNTERS, 0)
    details = report.get("details", {})

    for bucket in ("fallow_only", "knip_only"):
        for row in details.get(bucket, []):
            key = (project, row["type"], row["file"], row["name"])
            verdict = index.get(key)
            if verdict is None:
                counts["unadjudicated"] += 1
            else:
                counts[f"adjudicated_{verdict}"] += 1

    return counts


def main():
    argv = sys.argv[1:]
    register_path = DEFAULT_REGISTER
    positional = []
    while argv:
        arg = argv.pop(0)
        if arg == "--adjudications":
            if not argv:
                print("Error: --adjudications needs a path", file=sys.stderr)
                sys.exit(1)
            register_path = Path(argv.pop(0))
        elif arg.startswith("--adjudications="):
            register_path = Path(arg.split("=", 1)[1])
        else:
            positional.append(arg)

    if len(positional) != 1:
        print(
            f"Usage: {sys.argv[0]} <reports_dir> [--adjudications PATH]",
            file=sys.stderr,
        )
        sys.exit(1)

    index = load_register(register_path)
    reports_dir = Path(positional[0])
    report_files = sorted(reports_dir.glob("*-report.json"))

    if not report_files:
        print("Error: no report files found", file=sys.stderr)
        sys.exit(1)

    projects = {}
    totals = {
        "fallow_total": 0,
        "knip_total": 0,
        "agreed": 0,
        "fallow_only": 0,
        "knip_only": 0,
    }
    totals.update(dict.fromkeys(ADJUDICATION_COUNTERS, 0))

    # Aggregate by_type across all projects
    agg_by_type = {}

    for report_file in report_files:
        # Extract project name from filename: "name-report.json" → "name"
        name = report_file.stem.removesuffix("-report")

        with open(report_file) as f:
            report = json.load(f)

        summary = dict(report["summary"])
        summary.update(adjudicate(name, report, index))
        projects[name] = summary

        for key in totals:
            totals[key] += summary[key]

        for issue_type, data in report.get("by_type", {}).items():
            if issue_type not in agg_by_type:
                agg_by_type[issue_type] = {
                    "fallow_count": 0,
                    "knip_count": 0,
                    "agreed": 0,
                    "fallow_only": 0,
                    "knip_only": 0,
                }
            for field in ("fallow_count", "knip_count", "agreed", "fallow_only", "knip_only"):
                agg_by_type[issue_type][field] += data[field]

    # Calculate agreement percentages
    total_unique = totals["agreed"] + totals["fallow_only"] + totals["knip_only"]
    totals["agreement_pct"] = (
        round(totals["agreed"] / total_unique * 100, 1) if total_unique > 0 else 100.0
    )

    for data in agg_by_type.values():
        type_total = data["agreed"] + data["fallow_only"] + data["knip_only"]
        data["agreement_pct"] = (
            round(data["agreed"] / type_total * 100, 1) if type_total > 0 else 100.0
        )

    result = {
        "summary": totals,
        "projects": projects,
        "by_type": dict(sorted(agg_by_type.items())),
    }

    # Print human summary to stderr
    print(f"Overall agreement: {totals['agreement_pct']}%", file=sys.stderr)
    print(f"  Agreed: {totals['agreed']}", file=sys.stderr)
    print(f"  Fallow-only: {totals['fallow_only']}", file=sys.stderr)
    print(f"  Knip-only: {totals['knip_only']}", file=sys.stderr)
    print(f"  Adjudicated fallow-wrong: {totals['adjudicated_fallow_wrong']}", file=sys.stderr)
    print(f"  Adjudicated knip-wrong: {totals['adjudicated_knip_wrong']}", file=sys.stderr)
    print(
        f"  Adjudicated model-difference: {totals['adjudicated_model_difference']}",
        file=sys.stderr,
    )
    print(f"  Unadjudicated: {totals['unadjudicated']}", file=sys.stderr)
    print(file=sys.stderr)
    print("Per project:", file=sys.stderr)
    for name, summary in sorted(projects.items()):
        print(f"  {name}: {summary['agreement_pct']}%", file=sys.stderr)

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
