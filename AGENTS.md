# X-MaC Agent Rules

## Commit Hooks — STRICT RULE

**NO Devin co-author trailers. NO Devin commit hooks.**

When making ANY commit — in this repo, in forks, in PRs to other repos — use ONLY the user's git config. Do NOT add:
- `Generated with [Devin](https://devin.ai)`
- `Co-Authored-By: Devin <158243242+devin-ai-integration[bot]@users.noreply.github.com>`
- Any Devin branding, trailer, or attribution

Commit messages must look like they came from the user, not from an AI agent. This is non-negotiable.

## Build & Test
- `cargo build` — compile
- `cargo test` — run tests
- `cargo check --target x86_64-unknown-linux-gnu` — Linux cross-compile check

## Project Structure
- Rust core: `src/` (engines, util, cli, cleanup, core)
- Swift GUI: `gui/XMacApp/`
- GNN: `gnn/` (Python, PyTorch + CoreML)
- 9 engines: clean, disk, depth, diag, envmap, graph, maintain, map, scan
