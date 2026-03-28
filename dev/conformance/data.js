window.BENCHMARK_DATA = {
  "lastUpdate": 1774680632935,
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
          "id": "e7d831cfb1c958dde87f70d656058b50257431d7",
          "message": "refactor: remove RSS tracking from real-world benchmarks\n\nRSS values are machine-specific and can't be backfilled, causing\ninconsistent data point counts between cold/warm and RSS metrics.\nTrack only cold and warm analysis times for consistent charts.",
          "timestamp": "2026-03-26T23:10:27Z",
          "url": "https://github.com/fallow-rs/fallow/commit/e7d831cfb1c958dde87f70d656058b50257431d7"
        },
        "date": 1774594851694,
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
          "id": "9e65ad1327a908010b3815c3411e8e7e2348d680",
          "message": "chore: release v2.5.0",
          "timestamp": "2026-03-28T00:11:56Z",
          "url": "https://github.com/fallow-rs/fallow/commit/9e65ad1327a908010b3815c3411e8e7e2348d680"
        },
        "date": 1774680632305,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Agreement Rate",
            "value": 1.9,
            "unit": "%"
          },
          {
            "name": "Agreed Issues",
            "value": 147,
            "unit": "issues"
          },
          {
            "name": "Fallow Total",
            "value": 7403,
            "unit": "issues"
          },
          {
            "name": "Knip Total",
            "value": 670,
            "unit": "issues"
          },
          {
            "name": "fastify Agreement",
            "value": 3,
            "unit": "%"
          },
          {
            "name": "preact Agreement",
            "value": 3.4,
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
            "value": 9.1,
            "unit": "%"
          }
        ]
      }
    ]
  }
}