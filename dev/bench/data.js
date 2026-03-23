window.BENCHMARK_DATA = {
  "lastUpdate": 1774261280653,
  "repoUrl": "https://github.com/fallow-rs/fallow",
  "entries": {
    "Fallow Benchmarks": [
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "d004ea84285796d55c417b4de1c6c8accf38b283",
          "message": "fix: add Windows symlink support for workspace integration test\n\nThe workspace_project_discovers_workspace_packages test only created\nsymlinks on Unix, causing the cross-workspace resolution test to fail\non Windows CI. Add #[cfg(windows)] block using symlink_dir.",
          "timestamp": "2026-03-21T07:51:35+01:00",
          "tree_id": "ba8312d7a89e20c8fd3e62678774b8037c121544",
          "url": "https://github.com/fallow-rs/fallow/commit/d004ea84285796d55c417b4de1c6c8accf38b283"
        },
        "date": 1774076170050,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28905,
            "range": "± 296",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1094729,
            "range": "± 65986",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2631742,
            "range": "± 28315",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16732955,
            "range": "± 286442",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 283071,
            "range": "± 1756",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 215433,
            "range": "± 12290",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 919636,
            "range": "± 7154",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1724717,
            "range": "± 16493",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515696,
            "range": "± 27479",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4237058,
            "range": "± 34977",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2529488,
            "range": "± 12907",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "1eca935f0c37bd0f61588d8b81994f7200ff1f92",
          "message": "fix: resolve CI failures across typos, cargo-deny, docs, MSRV, and clippy\n\n- Add _typos.toml to exclude vscode dist, fixtures, and allow legitimate\n  identifiers (ue, ba, caf, fpr) used in variable names and test data\n- Update deny.toml for cargo-deny v2: remove deprecated unmaintained/yanked\n  fields, add MPL-2.0 and CC0-1.0 licenses, ignore bincode advisory\n- Fix rustdoc errors: backtick-wrap Vec<String> in doc comment, remove\n  intra-doc link to private FallowErrorKind\n- Bump MSRV from 1.85 to 1.92 (required by oxc 0.120.0 dependencies)\n- Fix clippy collapsible_if in mcp server binary resolution",
          "timestamp": "2026-03-21T08:06:01+01:00",
          "tree_id": "c3b261347252a51baa81fd47314eb891fa0b905e",
          "url": "https://github.com/fallow-rs/fallow/commit/1eca935f0c37bd0f61588d8b81994f7200ff1f92"
        },
        "date": 1774076995745,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28224,
            "range": "± 179",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1093349,
            "range": "± 24320",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2631597,
            "range": "± 30143",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16925857,
            "range": "± 116467",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 281440,
            "range": "± 1740",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2022,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206180,
            "range": "± 28084",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 918450,
            "range": "± 15578",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1738861,
            "range": "± 21186",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515251,
            "range": "± 9792",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4264046,
            "range": "± 43812",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2531112,
            "range": "± 117221",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "ab1cb8f4b357569ef53b221b8277eb3ab92a4b3f",
          "message": "chore: release v1.0.4",
          "timestamp": "2026-03-21T08:12:55+01:00",
          "tree_id": "480c76589a8dfcdba3b7ba43894b590b9b203f26",
          "url": "https://github.com/fallow-rs/fallow/commit/ab1cb8f4b357569ef53b221b8277eb3ab92a4b3f"
        },
        "date": 1774077424039,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 22953,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 887820,
            "range": "± 16555",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 1960347,
            "range": "± 27516",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 12501452,
            "range": "± 461281",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 249964,
            "range": "± 2408",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2756,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 161948,
            "range": "± 11680",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 745188,
            "range": "± 13662",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1528366,
            "range": "± 29594",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 471679,
            "range": "± 27134",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3547683,
            "range": "± 26830",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2039585,
            "range": "± 8631",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a0ceeec375bb55b10fa3d7acbda1ab21b1b4c76b",
          "message": "chore(deps): bump toml from 1.0.6+spec-1.1.0 to 1.0.7+spec-1.1.0 (#13)\n\nBumps [toml](https://github.com/toml-rs/toml) from 1.0.6+spec-1.1.0 to 1.0.7+spec-1.1.0.\n- [Commits](https://github.com/toml-rs/toml/compare/toml-v1.0.6...toml-v1.0.7)\n\n---\nupdated-dependencies:\n- dependency-name: toml\n  dependency-version: 1.0.7+spec-1.1.0\n  dependency-type: direct:production\n  update-type: version-update:semver-patch\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-21T08:42:30+01:00",
          "tree_id": "5995b5a095906106874f938357a552f7241a2aa8",
          "url": "https://github.com/fallow-rs/fallow/commit/a0ceeec375bb55b10fa3d7acbda1ab21b1b4c76b"
        },
        "date": 1774079167477,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28013,
            "range": "± 1542",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1109778,
            "range": "± 28362",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2618050,
            "range": "± 24478",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16834878,
            "range": "± 137456",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 281766,
            "range": "± 1241",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1990,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203374,
            "range": "± 3745",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 903442,
            "range": "± 12344",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1694903,
            "range": "± 33220",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 511894,
            "range": "± 25202",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4199247,
            "range": "± 46821",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2530292,
            "range": "± 9772",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "f23f31f0657152fdcc2adf227d4e664512f7cd66",
          "message": "chore: release v1.1.0",
          "timestamp": "2026-03-21T10:16:50+01:00",
          "tree_id": "607474b552ef8442f312393c8f431e191ef771f7",
          "url": "https://github.com/fallow-rs/fallow/commit/f23f31f0657152fdcc2adf227d4e664512f7cd66"
        },
        "date": 1774084837859,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 29018,
            "range": "± 2073",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1082908,
            "range": "± 142744",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2596059,
            "range": "± 37435",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16760486,
            "range": "± 168180",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 279909,
            "range": "± 1926",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1961,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201052,
            "range": "± 723",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 897912,
            "range": "± 10468",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1694621,
            "range": "± 20397",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514790,
            "range": "± 9439",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4179917,
            "range": "± 109824",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2544909,
            "range": "± 23585",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "2a4d74cfd00d5ec79733ff08e31dc311961a9ffc",
          "message": "fix: restrict ModuleNode size assertion to Unix (PathBuf differs on Windows)",
          "timestamp": "2026-03-21T10:26:39+01:00",
          "tree_id": "f83a7d02feecd2549717e3d56cfcae87576739a0",
          "url": "https://github.com/fallow-rs/fallow/commit/2a4d74cfd00d5ec79733ff08e31dc311961a9ffc"
        },
        "date": 1774085422873,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28324,
            "range": "± 1425",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1093204,
            "range": "± 62863",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2611032,
            "range": "± 32646",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16721651,
            "range": "± 269414",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 276516,
            "range": "± 1569",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1966,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201196,
            "range": "± 3506",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 897980,
            "range": "± 9814",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1690988,
            "range": "± 16746",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515760,
            "range": "± 28358",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4363817,
            "range": "± 94257",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2497857,
            "range": "± 60394",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "ad3c6ec83c75aaa3303cb67814d3e569518a66dc",
          "message": "fix: changelog, homepage URL, npm binary fallback, docs accuracy\n\n- Add missing CHANGELOG.md entry for v1.1.0\n- Change homepage from fallow.tools to docs.fallow.tools\n- Add try/catch around detect-libc in npm bin wrapper (fallback to musl)\n- Add workflow_dispatch trigger to Miri CI\n- Fix schema.rs: mark unused-enum-member as fixable, add markdown format\n- Fix AGENTS.md: accurate compact format description, schema_version field\n- Fix BENCHMARKS.md: correct bench target names\n- Fix docs/positioning.md: remove complexity hotspot claims\n- Fix docs/plugin-authoring.md: external plugins support detection logic",
          "timestamp": "2026-03-21T18:45:13+01:00",
          "tree_id": "a9912c446b8b8d77e52c032969f80141e0308f5e",
          "url": "https://github.com/fallow-rs/fallow/commit/ad3c6ec83c75aaa3303cb67814d3e569518a66dc"
        },
        "date": 1774115339798,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28809,
            "range": "± 542",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1086420,
            "range": "± 9061",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2621487,
            "range": "± 30688",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16671957,
            "range": "± 219688",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 277337,
            "range": "± 815",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1996,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 200943,
            "range": "± 1073",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 895631,
            "range": "± 9819",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1684273,
            "range": "± 17178",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 510033,
            "range": "± 27569",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4165592,
            "range": "± 46352",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2464431,
            "range": "± 6629",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "51db3d56712e7df6443a60ed680fc8a112b6137f",
          "message": "fix: escape [default] in rustdoc comment to fix doc build\n\nRustdoc interprets `[default]` as an intra-doc link. Escape the\nbrackets to prevent the broken-intra-doc-links warning.",
          "timestamp": "2026-03-21T18:49:15+01:00",
          "tree_id": "0f223d64ca2f5644db05b4232d775f4f336e6a35",
          "url": "https://github.com/fallow-rs/fallow/commit/51db3d56712e7df6443a60ed680fc8a112b6137f"
        },
        "date": 1774115579195,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28527,
            "range": "± 1066",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1088595,
            "range": "± 12908",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2622348,
            "range": "± 43970",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16740702,
            "range": "± 286464",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 280592,
            "range": "± 2695",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1968,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201917,
            "range": "± 19095",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 895986,
            "range": "± 12195",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1689452,
            "range": "± 31547",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515649,
            "range": "± 10111",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4165745,
            "range": "± 35280",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2473687,
            "range": "± 3215",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "ed499c258e70639bd923c65028dbc18026af6aad",
          "message": "refactor: adopt Oxc-style Miri CI pattern\n\n- Pin nightly toolchain (nightly-2026-03-20) to avoid Miri regressions\n- Use #[cfg(not(miri))] source guards instead of --skip CLI flags\n  for parser-heavy tests (visitor.rs, tests/ integration module)\n- Add --lib --tests to skip doctests (unsupported under Miri)\n- Add permissions: {} deny-all baseline\n- Keep suppress, sfc, css unit tests running (no Oxc parser calls)",
          "timestamp": "2026-03-21T21:02:46+01:00",
          "tree_id": "31dc901904139bccec698ff5be745118fc199e6c",
          "url": "https://github.com/fallow-rs/fallow/commit/ed499c258e70639bd923c65028dbc18026af6aad"
        },
        "date": 1774123591530,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28634,
            "range": "± 346",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1113739,
            "range": "± 30215",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2646637,
            "range": "± 28864",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17006620,
            "range": "± 631111",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 279535,
            "range": "± 3409",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1957,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201454,
            "range": "± 5988",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 898000,
            "range": "± 6391",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1685676,
            "range": "± 25387",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519964,
            "range": "± 30096",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4200984,
            "range": "± 77015",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2496070,
            "range": "± 17527",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "b475cfc20fe03c88353288ae3ba1d22152de4836",
          "message": "chore: release v1.2.0",
          "timestamp": "2026-03-22T12:57:27+01:00",
          "tree_id": "089417efd2637f2506a27a3c55aba1b955015d20",
          "url": "https://github.com/fallow-rs/fallow/commit/b475cfc20fe03c88353288ae3ba1d22152de4836"
        },
        "date": 1774181577468,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41033,
            "range": "± 358",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1105231,
            "range": "± 17303",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2668846,
            "range": "± 34236",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17859815,
            "range": "± 206342",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 296438,
            "range": "± 2060",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1987,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 200811,
            "range": "± 3676",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 902521,
            "range": "± 66830",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1705331,
            "range": "± 85196",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 512037,
            "range": "± 35313",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4308773,
            "range": "± 54348",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2517511,
            "range": "± 11238",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "fb09d1a8465e2cc8791203b9b5bcb9a6e687a8ab",
          "message": "feat: workspace perf fix, optionalDependencies, overload dedup, re-export optimization\n\nPerformance:\n- Workspace discovery: check package.json before canonicalize(), reducing\n  syscalls from 759 to ~20 on vite (245ms → ~30ms for that stage)\n- Re-export chain: replace ExportName::to_string() allocations with\n  zero-alloc matches_str() comparisons in hot inner loops\n\nAccuracy:\n- Parse optionalDependencies from package.json with full pipeline support:\n  detection, rules (unused-optional-dependencies), all report formats\n  (human, json, sarif, compact, markdown), auto-fix, LSP diagnostics\n- Deduplicate TypeScript function overload signatures during extraction:\n  `export function foo(): void; export function foo(x: string): string;\n  export function foo(x?: string) {}` produces 1 export, not 3",
          "timestamp": "2026-03-22T16:22:45+01:00",
          "tree_id": "1432c9685bf8d4d8be2ecc6e249ebbae2e5523e7",
          "url": "https://github.com/fallow-rs/fallow/commit/fb09d1a8465e2cc8791203b9b5bcb9a6e687a8ab"
        },
        "date": 1774193202166,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39714,
            "range": "± 1368",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1105867,
            "range": "± 27145",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2650603,
            "range": "± 40578",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16958678,
            "range": "± 257266",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106036,
            "range": "± 1287",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1968,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201353,
            "range": "± 873",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 898148,
            "range": "± 10129",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1687186,
            "range": "± 19317",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 543181,
            "range": "± 28003",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4173677,
            "range": "± 40550",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2480920,
            "range": "± 5001",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "523e701ba71f8eaf95a961258aaa0a30de2c4a6a",
          "message": "chore: release v1.3.0",
          "timestamp": "2026-03-22T17:52:34+01:00",
          "tree_id": "5401d0e941ba62b3a5353dc552c34d21b67c9b3a",
          "url": "https://github.com/fallow-rs/fallow/commit/523e701ba71f8eaf95a961258aaa0a30de2c4a6a"
        },
        "date": 1774198593588,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 40870,
            "range": "± 486",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1110631,
            "range": "± 55920",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2669147,
            "range": "± 27573",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17348720,
            "range": "± 177463",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106177,
            "range": "± 368",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2006,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206701,
            "range": "± 6931",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 929181,
            "range": "± 19641",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1732569,
            "range": "± 23589",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 521770,
            "range": "± 13248",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4352772,
            "range": "± 64480",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2583007,
            "range": "± 31021",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "e77bf1449e6dd9d4b1ae81f1f1da8d461f506c77",
          "message": "test: add optionalDependencies integration test and snapshot coverage",
          "timestamp": "2026-03-22T19:33:53+01:00",
          "tree_id": "988af83b41a1b6ff0e13d65d67f1a9210910cbba",
          "url": "https://github.com/fallow-rs/fallow/commit/e77bf1449e6dd9d4b1ae81f1f1da8d461f506c77"
        },
        "date": 1774204664382,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 33636,
            "range": "± 842",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 895947,
            "range": "± 17580",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 1997106,
            "range": "± 27833",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 12346281,
            "range": "± 340146",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 96610,
            "range": "± 1133",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2853,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 162481,
            "range": "± 829",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 733975,
            "range": "± 3330",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1526969,
            "range": "± 24412",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 468350,
            "range": "± 15047",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3485392,
            "range": "± 31405",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2034593,
            "range": "± 12756",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "5e83aee8c5a7bf37afb505e7ccd535a144b97211",
          "message": "chore: release v1.3.1",
          "timestamp": "2026-03-23T00:13:45+01:00",
          "tree_id": "1a86251710998d54f51f3434b4850782b304a780",
          "url": "https://github.com/fallow-rs/fallow/commit/5e83aee8c5a7bf37afb505e7ccd535a144b97211"
        },
        "date": 1774221550569,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41363,
            "range": "± 1594",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1185336,
            "range": "± 18709",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2803866,
            "range": "± 25173",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17781842,
            "range": "± 217479",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103301,
            "range": "± 1123",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1974,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 222659,
            "range": "± 28984",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1001027,
            "range": "± 6219",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1783626,
            "range": "± 18731",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548513,
            "range": "± 20921",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4375573,
            "range": "± 46769",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2875973,
            "range": "± 7276",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "09a5da5de787845af5e061fb7a8117c2bbddd6fe",
          "message": "fix: fix conformance workflow JSON corruption and update CI action versions\n\n- Remove incorrect `2>&1 >&2` redirect in conformance run.sh that leaked\n  Python's stderr (human summary) into stdout, corrupting the JSON report\n- Redirect npm install stdout to /dev/null to prevent it contaminating output\n- Add JSON validation in conformance workflow before parsing report\n- Improve error handling: show stderr + raw report content on failure\n- Update action versions to fix Node.js 20 deprecation warnings:\n  checkout v4->v6, setup-node v4->v6, upload-artifact v4->v7, cache v4->v5",
          "timestamp": "2026-03-23T10:18:43+01:00",
          "tree_id": "a8c96a56a67f41c97956a9444e7d4b08e605e7a2",
          "url": "https://github.com/fallow-rs/fallow/commit/09a5da5de787845af5e061fb7a8117c2bbddd6fe"
        },
        "date": 1774257757869,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 40298,
            "range": "± 401",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1188528,
            "range": "± 8829",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2815230,
            "range": "± 34487",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17909293,
            "range": "± 121232",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101937,
            "range": "± 3982",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226884,
            "range": "± 4515",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1015316,
            "range": "± 9109",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1787795,
            "range": "± 26285",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548575,
            "range": "± 11394",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4441250,
            "range": "± 30945",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2819656,
            "range": "± 7635",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "2a8ab7e14e1bf69949bfc2d2ffe6c7f0243a349e",
          "message": "feat: add progress spinners to analysis pipeline and improve watch mode UX\n\nWire AnalysisProgress spinners into the analysis pipeline stages (discover,\nplugins, parse, resolve, graph, analyze). Progress is shown when stderr is a\nTTY and output format is human-readable — suppressed for --quiet, --format\njson/sarif, and non-TTY environments.\n\nAdd quiet field to ResolvedConfig to control progress bar visibility from\nthe CLI layer. Tracing level set to WARN when spinners are active to prevent\nstderr corruption between indicatif and tracing-subscriber output.",
          "timestamp": "2026-03-23T10:23:06+01:00",
          "tree_id": "86178de0d493c8c33df5a0597670b50f0b1f180a",
          "url": "https://github.com/fallow-rs/fallow/commit/2a8ab7e14e1bf69949bfc2d2ffe6c7f0243a349e"
        },
        "date": 1774258026351,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41436,
            "range": "± 1146",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1242692,
            "range": "± 16947",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2857671,
            "range": "± 49539",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18317242,
            "range": "± 341391",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103779,
            "range": "± 2334",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1998,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226963,
            "range": "± 4129",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1025613,
            "range": "± 10090",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1791847,
            "range": "± 20386",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 544173,
            "range": "± 34155",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4505971,
            "range": "± 51940",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2850329,
            "range": "± 10663",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "bca6412b007042e650249de0915e4396a10b50a3",
          "message": "refactor: split 5 largest functions into focused helpers\n\nExtract helpers from main() (258→50 lines), discover_workspaces() (216→12),\nrun_check() (215→130), build_markdown() (195→150), and\nvisit_variable_declaration() (155→40) to improve readability and\nmaintainability.",
          "timestamp": "2026-03-23T10:45:52+01:00",
          "tree_id": "355fd6771f769c139bfa535f81c21f0ed1f2ba4b",
          "url": "https://github.com/fallow-rs/fallow/commit/bca6412b007042e650249de0915e4396a10b50a3"
        },
        "date": 1774259415058,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 40795,
            "range": "± 336",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1217288,
            "range": "± 11736",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2829390,
            "range": "± 115474",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17983930,
            "range": "± 120978",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 110482,
            "range": "± 1687",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1967,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 225685,
            "range": "± 14712",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1013221,
            "range": "± 11195",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1790488,
            "range": "± 19914",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 552393,
            "range": "± 9521",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4455116,
            "range": "± 52943",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2830567,
            "range": "± 18186",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "bd1e9597c24d5083496cde7e016efa4246bcf52a",
          "message": "feat: add historical metric tracking and GitHub Pages dashboard\n\nAdd benchmark-action/github-action-benchmark tracking to allocs,\ncoverage, and conformance workflows alongside the existing bench\nworkflow. Each pushes to its own gh-pages directory (dev/allocs/,\ndev/coverage/, dev/conformance/) with auto-generated trend charts.\n\n- allocs.yml: track allocations over time (customSmallerIsBetter),\n  PR comments on allocation changes\n- coverage.yml: track coverage % over time (customBiggerIsBetter),\n  migrate badge push from fragile git stash to git worktree\n- conformance.yml: track agreement rate over time\n  (customBiggerIsBetter), upgrade permissions for gh-pages push\n- bench.yml: deploy dashboard index.html to gh-pages root\n- Add .github/pages/index.html dashboard linking to all 4 charts",
          "timestamp": "2026-03-23T10:55:20+01:00",
          "tree_id": "d760a210ccec7fe6e80dedc6efb46d2559f7201a",
          "url": "https://github.com/fallow-rs/fallow/commit/bd1e9597c24d5083496cde7e016efa4246bcf52a"
        },
        "date": 1774259960790,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41686,
            "range": "± 736",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1216368,
            "range": "± 23794",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2834890,
            "range": "± 39965",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 19263584,
            "range": "± 212691",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 111643,
            "range": "± 559",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1990,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226595,
            "range": "± 5647",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1014782,
            "range": "± 12373",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1804960,
            "range": "± 23651",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548446,
            "range": "± 18595",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4478689,
            "range": "± 55656",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2843045,
            "range": "± 16298",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "dc388543a716872d2e4999785ecff85eea6e903e",
          "message": "feat: add type-only-dependencies rule and fix formatter gaps\n\nAdd `type-only-dependencies` to RulesConfig (default: warn) so it is\nconfigurable like all other rules. Previously hardcoded to warn in\nhuman/SARIF output.\n\nAlso adds missing type_only_dependencies support to compact and SARIF\nformatters, deduplicates dependency formatting in human.rs, and\nextracts shared format_dependency helper in markdown.rs.",
          "timestamp": "2026-03-23T11:12:58+01:00",
          "tree_id": "d4a14d535e02845b2d8440f3066f06497e0664e6",
          "url": "https://github.com/fallow-rs/fallow/commit/dc388543a716872d2e4999785ecff85eea6e903e"
        },
        "date": 1774261032573,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 34263,
            "range": "± 355",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 957139,
            "range": "± 10910",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2047216,
            "range": "± 70302",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 11964887,
            "range": "± 107451",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 96828,
            "range": "± 2226",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2790,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 166613,
            "range": "± 1093",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 736787,
            "range": "± 4949",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1518108,
            "range": "± 31449",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 472348,
            "range": "± 14622",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3455833,
            "range": "± 26065",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2017862,
            "range": "± 5404",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "eb707bfebfe7aba7168b56eca51af00e4ee64ecb",
          "message": "fix: add missing type_only_dependencies field in snapshot test",
          "timestamp": "2026-03-23T11:15:50+01:00",
          "tree_id": "3a196e9777ef417ef043e1da34a35fde455dd63b",
          "url": "https://github.com/fallow-rs/fallow/commit/eb707bfebfe7aba7168b56eca51af00e4ee64ecb"
        },
        "date": 1774261279688,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39995,
            "range": "± 555",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1209190,
            "range": "± 11565",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2827997,
            "range": "± 109820",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17767536,
            "range": "± 144258",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 110916,
            "range": "± 871",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1978,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226496,
            "range": "± 7319",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1016377,
            "range": "± 31300",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1790561,
            "range": "± 29442",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 542113,
            "range": "± 4819",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4461184,
            "range": "± 42697",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2832225,
            "range": "± 6325",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}