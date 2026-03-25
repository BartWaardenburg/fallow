### Configuration options

| Setting | What it does |
|---------|-------------|
| **Issue Types** | Toggle individual checks on/off (unused files, exports, deps, etc.) |
| **Production mode** | Only analyze production code, exclude test/dev files |
| **Duplication mode** | Choose strictness: `strict`, `mild`, `weak`, or `semantic` |
| **Duplication threshold** | Minimum lines for a block to count as duplicate (default: 5) |

For project-wide config, create a `.fallowrc.json` in your project root with entry points, ignore patterns, and rule severity.
