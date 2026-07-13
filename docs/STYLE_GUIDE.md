# Style Guide

This document defines coding conventions for X-MaC. Follow these when contributing.

## Rust

### Formatting
- Run `cargo fmt` before committing. CI enforces `cargo fmt --check`.
- 4-space indentation, no tabs.
- Max line width: 100 characters (rustfmt default).

### Naming
- `snake_case` for functions, methods, variables, modules.
- `PascalCase` for types (structs, enums, traits).
- `SCREAMING_SNAKE_CASE` for constants.
- Module names are singular (`engine` not `engines`, unless it's the parent `engines/`).

### Error Handling
- Use `anyhow::Result` for application-level errors (CLI, main).
- Use `thiserror` for library-level errors that callers need to match on.
- Never use `.unwrap()` in production code. Use `?` or `.unwrap_or_default()`.
- `.unwrap()` is acceptable in tests.
- Don't over-nest `match` — use `?` and early returns.

### Imports
- Group imports: std → external crates → crate-internal.
- Use `use crate::` for internal imports.
- Remove unused imports (CI will fail on warnings).

### Comments
- Comments explain **why**, not **what**. The code already shows what.
- Use `///` for public API doc comments.
- Use `//` for inline comments.
- Don't leave commented-out code in the codebase.

### Async
- All engine methods are `async` (uses `async_trait`).
- Use `tokio` for the async runtime.
- Use `mpsc::channel` for finding streaming.
- Don't `.await` while holding a lock.

### Testing
- Tests live in `#[cfg(test)] mod tests` at the bottom of each file.
- Integration tests go in `tests/`.
- Test names: `test_<what>_<condition>` (e.g., `test_advisor_with_gaming_profile`).
- Use `assert!` for booleans, `assert_eq!` for equality, `assert!(x.contains("..."))` for strings.

### Platform Conditionals
- Use `#[cfg(target_os = "macos")]` for macOS-only code.
- Use `#[cfg(target_os = "linux")]` for Linux-only code.
- Always provide a fallback for the other platform (even if it's a no-op).
- Never `#[cfg(windows)]` — X-MaC doesn't target Windows.

## Swift

### Formatting
- 2-space indentation.
- Max line width: 120 characters.

### Naming
- `PascalCase` for types, protocols, structs, classes, enums.
- `camelCase` for functions, methods, variables, properties.
- View files end with `View.swift` (e.g., `ZenView.swift`).

### SwiftUI
- Use `@Published` for observable state in `ObservableObject` classes.
- Use `@State` for local view state.
- Use `@EnvironmentObject` for shared app state.
- Follow the `XTheme` style system — use `XCard`, `XSectionHeader`, `XTheme` colors.
- Don't hardcode colors — use `XTheme.neural*` color palette.

### Architecture
- `XMacRunner` is the bridge to the Rust binary. All CLI calls go through it.
- Views are stateless — they read from `XMacRunner` and render.
- Models in `Models.swift` are `Codable` for JSON parsing.

## Python (GNN)

### Formatting
- Follow PEP 8.
- Use `black` for formatting (line length 100).
- Type hints for all function signatures.

### Structure
- Training scripts at top level (`train.py`, `train_memory_gnn.py`).
- Model architecture in `model/`.
- Data generation in `data_generator.py`.
- CoreML export in `export_coreml.py`.

## Markdown

- 2-space indentation in lists.
- Use `#` for top-level headers, `##` for sections, `###` for subsections.
- Keep lines under 120 characters where possible.
- Use fenced code blocks with language tags (` ```rust `, ` ```bash `, etc.).
- Don't use HTML in markdown unless absolutely necessary.

## Git

### Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):
```
type(scope): description

feat(clean): add Docker image cache detection
fix(disk): correct APFS sparse file size calculation
docs(architecture): add Mermaid diagrams
```

### Branch Names
- `feat/<name>` — new features
- `fix/<name>` — bug fixes
- `docs/<name>` — documentation
- `refactor/<name>` — refactoring
- `chore/<name>` — maintenance
