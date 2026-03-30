---
paths:
  - "crates/lsp/**"
---

# fallow-lsp crate

Key modules:
- `main.rs` — LSP server setup, `LanguageServer` trait impl, event handling
- `diagnostics.rs` — Diagnostic generation for all issue types
- `code_actions.rs` — Quick-fix and refactor code actions
- `code_lens.rs` — Reference count Code Lens above export declarations
- `hover.rs` — Hover information showing export usage, unused status, and duplicate block locations
