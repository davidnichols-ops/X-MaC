# Examples

This directory contains example configurations and CLI usage patterns.

## Contents

- [`configs/`](configs/) — Sample `config.toml` files for different use cases:
  - [`default.toml`](configs/default.toml) — balanced everyday use
  - [`gaming.toml`](configs/gaming.toml) — aggressive memory cleanup for gaming
  - [`development.toml`](configs/development.toml) — build artifact cleanup for developers
  - [`conservative.toml`](configs/conservative.toml) — minimal intervention for production machines
- [`cli/`](cli/) — CLI usage examples and shell one-liners

## Using a Config

```bash
# Copy a sample config
cp examples/configs/development.toml ~/.config/xmac/config.toml

# Or use the built-in profile system
xmac config set-profile development
```
