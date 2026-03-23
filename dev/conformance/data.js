window.BENCHMARK_DATA = {
  "lastUpdate": 1774260107136,
  "repoUrl": "https://github.com/fallow-rs/fallow",
  "entries": {
    "Fallow Conformance": [
      {
        "commit": {
          "author": {
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg",
            "email": "bart@waardenburg.dev"
          },
          "committer": {
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg",
            "email": "bart@waardenburg.dev"
          },
          "id": "bd1e9597c24d5083496cde7e016efa4246bcf52a",
          "message": "feat: add historical metric tracking and GitHub Pages dashboard\n\nAdd benchmark-action/github-action-benchmark tracking to allocs,\ncoverage, and conformance workflows alongside the existing bench\nworkflow. Each pushes to its own gh-pages directory (dev/allocs/,\ndev/coverage/, dev/conformance/) with auto-generated trend charts.\n\n- allocs.yml: track allocations over time (customSmallerIsBetter),\n  PR comments on allocation changes\n- coverage.yml: track coverage % over time (customBiggerIsBetter),\n  migrate badge push from fragile git stash to git worktree\n- conformance.yml: track agreement rate over time\n  (customBiggerIsBetter), upgrade permissions for gh-pages push\n- bench.yml: deploy dashboard index.html to gh-pages root\n- Add .github/pages/index.html dashboard linking to all 4 charts",
          "timestamp": "2026-03-23T09:55:20Z",
          "url": "https://github.com/fallow-rs/fallow/commit/bd1e9597c24d5083496cde7e016efa4246bcf52a"
        },
        "date": 1774260106606,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Agreement Rate",
            "value": 66.7,
            "unit": "%"
          },
          {
            "name": "Agreed Issues",
            "value": 6,
            "unit": "issues"
          },
          {
            "name": "Fallow Total",
            "value": 9,
            "unit": "issues"
          },
          {
            "name": "Knip Total",
            "value": 6,
            "unit": "issues"
          }
        ]
      }
    ]
  }
}