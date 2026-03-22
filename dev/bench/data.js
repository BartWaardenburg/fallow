window.BENCHMARK_DATA = {
  "lastUpdate": 1774193203007,
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
      }
    ]
  }
}