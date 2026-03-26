window.BENCHMARK_DATA = {
  "lastUpdate": 1774562964177,
  "repoUrl": "https://github.com/fallow-rs/fallow",
  "entries": {
    "Fallow Real-World Benchmarks": [
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
        "date": 1774562963724,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "preact (cold)",
            "value": 45,
            "unit": "ms"
          },
          {
            "name": "preact (warm)",
            "value": 45,
            "unit": "ms"
          },
          {
            "name": "preact RSS",
            "value": 17899520,
            "unit": "bytes"
          },
          {
            "name": "fastify (cold)",
            "value": 56,
            "unit": "ms"
          },
          {
            "name": "fastify (warm)",
            "value": 47,
            "unit": "ms"
          },
          {
            "name": "fastify RSS",
            "value": 22134784,
            "unit": "bytes"
          },
          {
            "name": "zod (cold)",
            "value": 44,
            "unit": "ms"
          },
          {
            "name": "zod (warm)",
            "value": 36,
            "unit": "ms"
          },
          {
            "name": "zod RSS",
            "value": 18202624,
            "unit": "bytes"
          },
          {
            "name": "vue-core (cold)",
            "value": 115,
            "unit": "ms"
          },
          {
            "name": "vue-core (warm)",
            "value": 104,
            "unit": "ms"
          },
          {
            "name": "vue-core RSS",
            "value": 29380608,
            "unit": "bytes"
          },
          {
            "name": "svelte (cold)",
            "value": 441,
            "unit": "ms"
          },
          {
            "name": "svelte (warm)",
            "value": 424,
            "unit": "ms"
          },
          {
            "name": "svelte RSS",
            "value": 60620800,
            "unit": "bytes"
          },
          {
            "name": "query (cold)",
            "value": 340,
            "unit": "ms"
          },
          {
            "name": "query (warm)",
            "value": 327,
            "unit": "ms"
          },
          {
            "name": "query RSS",
            "value": 70361088,
            "unit": "bytes"
          },
          {
            "name": "vite (cold)",
            "value": 227,
            "unit": "ms"
          },
          {
            "name": "vite (warm)",
            "value": 212,
            "unit": "ms"
          },
          {
            "name": "vite RSS",
            "value": 37625856,
            "unit": "bytes"
          },
          {
            "name": "next.js (cold)",
            "value": 1812,
            "unit": "ms"
          },
          {
            "name": "next.js (warm)",
            "value": 1728,
            "unit": "ms"
          },
          {
            "name": "next.js RSS",
            "value": 185729024,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}