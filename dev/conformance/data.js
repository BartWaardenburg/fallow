window.BENCHMARK_DATA = {
"lastUpdate": 1774565821657,
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
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "6bb2e3800d8db809e7feaf451a9fa465b4cdf52a",
"message": "v1.6.1: chore: release v1.6.1",
"timestamp": "2026-03-24T13:19:35+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/6bb2e3800d8db809e7feaf451a9fa465b4cdf52a"
},
"date": 1774354775000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "8002592f5bb2e88db5557e058e9aac140b17d093",
"message": "v1.7.0: chore: release v1.7.0",
"timestamp": "2026-03-24T15:23:48+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/8002592f5bb2e88db5557e058e9aac140b17d093"
},
"date": 1774362228000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "3ccc6180252b9ce972442f375bed1649da4a8a0d",
"message": "v1.8.0: chore: release v1.8.0",
"timestamp": "2026-03-24T23:24:41+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/3ccc6180252b9ce972442f375bed1649da4a8a0d"
},
"date": 1774391081000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "ae00a48d67123829db61ac6123e5e8fda9297495",
"message": "v1.8.1: chore: release v1.8.1",
"timestamp": "2026-03-25T09:48:26+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/ae00a48d67123829db61ac6123e5e8fda9297495"
},
"date": 1774428506000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "e984a07c698bb4d63d06896fba119a37b5e316e0",
"message": "v1.9.0: fix: rename GitHub Action to reflect broader codebase health positioning",
"timestamp": "2026-03-25T17:44:35+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/e984a07c698bb4d63d06896fba119a37b5e316e0"
},
"date": 1774457075000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "818667f5f407a0210e7980d74fd36c3a5e65e503",
"message": "v2.0.0: chore: release v2.0.0",
"timestamp": "2026-03-25T18:54:56+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/818667f5f407a0210e7980d74fd36c3a5e65e503"
},
"date": 1774461296000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "285109152aeedafa408ccb0ba7d34622c22255a9",
"message": "v2.0.1: chore: release v2.0.1",
"timestamp": "2026-03-25T22:23:18+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/285109152aeedafa408ccb0ba7d34622c22255a9"
},
"date": 1774473798000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
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
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "a3701ac62f58099b431db018ab29fa08e7a34b9c",
"message": "v2.1.0: chore: release v2.1.0",
"timestamp": "2026-03-26T09:28:15+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/a3701ac62f58099b431db018ab29fa08e7a34b9c"
},
"date": 1774513695000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "65a26631b2fa3979f1f05857fa3044d361b1afa2",
"message": "v2.2.0: chore: release v2.2.0",
"timestamp": "2026-03-26T16:26:30+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/65a26631b2fa3979f1f05857fa3044d361b1afa2"
},
"date": 1774538790000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
"unit": "issues"
}
]
},
{
"commit": {
"author": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"committer": {
"name": "Bart Waardenburg",
"username": "",
"email": "bart@waardenburg.dev"
},
"id": "246502e30e80a5606cfbd6341bf925ff78136a29",
"message": "v2.2.1: chore: release v2.2.1",
"timestamp": "2026-03-26T21:04:40+01:00",
"url": "https://github.com/fallow-rs/fallow/commit/246502e30e80a5606cfbd6341bf925ff78136a29"
},
"date": 1774555480000,
"tool": "customBiggerIsBetter",
"benches": [
{
"name": "preact Agreement",
"value": 2.9,
"unit": "%"
},
{
"name": "fastify Agreement",
"value": 3.1,
"unit": "%"
},
{
"name": "zod Agreement",
"value": 9.1,
"unit": "%"
},
{
"name": "svelte Agreement",
"value": 0.3,
"unit": "%"
},
{
"name": "query Agreement",
"value": 0.0,
"unit": "%"
},
{
"name": "Agreement Rate",
"value": 1.8,
"unit": "%"
},
{
"name": "Agreed Issues",
"value": 142,
"unit": "issues"
},
{
"name": "Fallow Total",
"value": 7256,
"unit": "issues"
},
{
"name": "Knip Total",
"value": 671,
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
};