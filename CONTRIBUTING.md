# Contributing to X-MaC

Thank you for your interest in contributing to X-MaC! This document covers everything you need to get started.

## Quick Start

```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC
cargo build          # compile the Rust engine
cargo test           # run the test suite (327+ tests)
cd gui/XMacApp && swift build   # build the SwiftUI app
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed setup instructions.

## Project Layout

```
src/          Rust scan engine, CLI, cleanup, intelligence suite
gui/          SwiftUI macOS app
gnn/          PyTorch GNN model + CoreML export
tests/        Rust integration tests
docs/         Architecture docs, diagrams
scripts/      Helper scripts (lint, format, build)
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full system design.

## How to Contribute

### Reporting Bugs

1. Search [existing issues](https://github.com/davidnichols-ops/X-MaC/issues) to avoid duplicates
2. Open a bug report using the issue template
3. Include: macOS/Linux version, Mac model (Intel/Apple Silicon), `xmac --version` output, and steps to reproduce

### Suggesting Features

1. Open a feature request using the issue template
2. Describe the use case and expected behavior
3. If you have a proposed implementation, mention it

### Submitting Pull Requests

1. Fork the repo and create a branch from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
2. Make your changes. Follow the [style guide](docs/STYLE_GUIDE.md).
3. Ensure all checks pass:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```
4. For GUI changes, also verify:
   ```bash
   cd gui/XMacApp && swift build
   ```
5. Write a clear commit message (see conventions below).
6. Open a PR referencing any related issues.

## Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
type(scope): description

feat(clean): add Docker image cache detection
fix(disk): correct APFS sparse file size calculation
docs(readme): update installation instructions
test(advisor): add profile variation tests
refactor(config): simplify config loading
chore: update dependencies
```

**Types:** `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`, `ci`

**Scopes (optional):** `clean`, `disk`, `maintain`, `map`, `depth`, `conflict`, `envmap`, `graph`, `optimize`, `config`, `intelligence`, `gui`, `gnn`, `cli`

## Adding a New Scan Engine (Rust)

1. Create `src/engines/<name>/mod.rs` and implement the `Engine` trait
2. Add a new `EngineId` variant in `src/core/types.rs`
3. Register in `src/engines/mod.rs`
4. Wire into `src/cli/args.rs` and the relevant `run_*` function in `src/main.rs`
5. Add integration tests in `tests/`
6. Update `docs/ARCHITECTURE.md` with the new engine

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the engine trait contract.

## Adding a New GUI View (SwiftUI)

1. Create `gui/XMacApp/Sources/XMacApp/<Name>View.swift`
2. Follow the `XTheme` / `XCard` / `XSectionHeader` style system
3. Add a case to `AppSection` in `ContentView.swift`
4. Wire up the runner call in `XMacRunner.swift`

## Testing Expectations

- Every new feature needs tests
- Bug fixes should include a regression test
- Aim for tests that fail without your fix and pass with it
- Run `cargo test` before opening a PR

## Branching Strategy

- `main` — stable, always builds, always passes tests
- Feature branches — `feat/<name>`, `fix/<name>`, `docs/<name>`
- Release tags — `v2.1.0`, `v2.2.0`, etc.

## Code of Conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Be respectful, constructive, and welcoming.

## License

By contributing, you agree that your contributions are licensed under the MIT License.
