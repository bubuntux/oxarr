# Agent Guidelines (`oxarr`)

## Documentation & Roadmap

- **Architecture (North Star):** `docs/architecture.md` describes evergreen system design, crate boundaries, data flow, and technology decisions. Keep it strictly design-focused — do not add todo lists, checkboxes, or milestone tracking there.
- **Roadmap, Milestones & Tasks:** Tracked in GitHub Milestones and Issues on `bubuntux/oxarr` (`https://github.com/bubuntux/oxarr`).
  - List milestones: `gh api repos/bubuntux/oxarr/milestones`
  - List open issues: `gh issue list -R bubuntux/oxarr`
  - Filter by milestone: `gh issue list -R bubuntux/oxarr --milestone "<milestone title>"`
  - Inspect an issue: `gh issue view <number> -R bubuntux/oxarr`
  - Sandbox note: `gh` reads `~/.config/gh` and calls `api.github.com`. If terminal sandboxing blocks access to either, run the `gh` command outside the sandbox (or ask the user to grant read access to `~/.config/gh` and network access to `api.github.com` in project settings).

## Development Environment

- **Dev shell:** `devenv` (`devenv.nix`) provides the pinned Rust toolchain (`wasm32-unknown-unknown` target), `cargo-nextest`, `cargo-watch`, `sqlx-cli`, `sqlite`, and `gh`. (In sandboxes where `/run/user` is read-only, run `XDG_RUNTIME_DIR=/tmp devenv shell -- <cmd>`.)
