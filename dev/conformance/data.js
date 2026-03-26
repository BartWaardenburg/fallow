window.BENCHMARK_DATA = {
  "lastUpdate": 1774561393566,
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
      },
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
          "id": "48650d8d08b8185490c17e10497098a52cc3e971",
          "message": "chore: release v2.0.1",
          "timestamp": "2026-03-25T21:23:18Z",
          "url": "https://github.com/fallow-rs/fallow/commit/48650d8d08b8185490c17e10497098a52cc3e971"
        },
        "date": 1774508415306,
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
      },
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
          "id": "6466c29f10f928356aaadbd9c519cdf9565e5716",
          "message": "fix: suppress npm/pnpm install stdout leaking into JSON reports\n\nnpm install output was written to stdout, polluting the JSON report\ncaptured by the workflow. Redirect both stdout and stderr to /dev/null.",
          "timestamp": "2026-03-26T21:38:39Z",
          "url": "https://github.com/fallow-rs/fallow/commit/6466c29f10f928356aaadbd9c519cdf9565e5716"
        },
        "date": 1774561392542,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Agreement Rate",
            "value": 1.8,
            "unit": "%"
          },
          {
            "name": "Agreed Issues",
            "value": 141,
            "unit": "issues"
          },
          {
            "name": "Fallow Total",
            "value": 7257,
            "unit": "issues"
          },
          {
            "name": "Knip Total",
            "value": 670,
            "unit": "issues"
          },
          {
            "name": "fastify Agreement",
            "value": 3.1,
            "unit": "%"
          },
          {
            "name": "preact Agreement",
            "value": 2.9,
            "unit": "%"
          },
          {
            "name": "query Agreement",
            "value": 0,
            "unit": "%"
          },
          {
            "name": "svelte Agreement",
            "value": 0.3,
            "unit": "%"
          },
          {
            "name": "zod Agreement",
            "value": 9,
            "unit": "%"
          }
        ]
      }
    ]
  }
}