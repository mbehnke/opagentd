# opagentd — Agents

## Project Overview

`opagentd` is a privileged system daemon that securely executes LLM-driven agents on Linux systems. Every command passes through a validation and approval pipeline before execution.

- **Language**: Rust (edition 2021)
- **Build**: Cargo workspace + Makefile
- **License**: MIT

## Project Structure

```
opagentd/                  # Workspace root
├── Cargo.toml             # Workspace manifest
├── Makefile               # build, install, test, clean
├── opagent-core/          # Shared library
│   ├── tests/
│   │   └── integration.rs # Integration tests
│   └── src/
│       ├── lib.rs
│       ├── config.rs      # TOML config loading
│       ├── security.rs    # Security levels, operation types, validation
│       ├── message.rs     # IPC message types (serde)
│       └── agents/        # Agent implementations
│           ├── mod.rs     # AgentExecutor trait
│           ├── shell.rs   # Shell command execution
│           ├── file.rs    # File read/write/delete
│           └── git.rs     # Git operations
├── opagentd/              # Daemon binary
│   └── src/main.rs        # Unix-domain socket server loop
├── opagentctl/            # CLI client binary
│   └── src/main.rs        # clap-based CLI, connects to socket
├── opagent-api/           # Optional REST bridge
│   └── src/main.rs
├── systemd/
│   └── opagentd.service   # systemd unit file
├── config/
│   └── opagentd.toml      # Default configuration
└── tests/
    └── integration.rs     # Integration tests
```

## Agent Types

| Agent | Description |
|-------|-------------|
| Shell Agent | Executes shell commands (validated against blocklist) |
| File Agent | Reads/writes files within allowed paths |
| Git Agent | Performs git operations (dangerous commands require confirmation) |

## Security Levels

1. **auto** — Trusted, predefined commands; execute immediately
2. **confirm** — Requires approval via `opagentctl approve`
3. **deny** — Blocked for unknown/risky operations (default for unrecognized)

### Validation Pipeline

```
Submit → SecurityLevel → auto → Execute
                       → confirm → Queue → approve/deny → Execute
                       → deny → Reject
```

## Build Instructions

```bash
# Build all binaries
make build

# Run tests
make test

# Install to system (requires root)
sudo make install

# Uninstall
sudo make uninstall

# Clean build artifacts
make clean
```

### Requirements

- Rust toolchain (cargo, rustc) — edition 2021
- Linux with systemd (for service management)
- `/run` tmpfs (for runtime socket)

## Configuration

Default config at `/etc/opagentd/config.toml`:

```toml
[security]
default_level = "confirm"
allowed_paths = ["/home", "/tmp", "/var/tmp"]
denied_commands = ["rm -rf /", "dd", "> /dev/sda", "mkfs"]

[socket]
path = "/run/opagentd/opagentd.sock"
permissions = 0o660

[logging]
level = "info"
audit_log = "/var/log/opagentd/audit.jsonl"

[agent]
shell_enabled = true
file_enabled = true
git_enabled = true

[llm]
provider = "deepseek"
model = "deepseek-chat"
# base_url = "https://api.deepseek.com"   # optional override
# api_key = "sk-..."                       # or env: DEEPSEEK_API_KEY
temperature = 0.7
# max_tokens = 4096
enabled = true
```

### File System Paths

| Path | Purpose |
|------|---------|
| `/etc/opagentd/config.toml` | Configuration |
| `/run/opagentd/opagentd.sock` | Unix domain socket |
| `/var/log/opagentd/` | Audit logs |
| `/usr/local/bin/opagentd` | Daemon binary |
| `/usr/local/bin/opagentctl` | CLI binary |

## Code Conventions

- **Error handling**: Use `thiserror` for library errors, `anyhow` for binaries
- **Logging**: Use `tracing` crate; daemon logs to journald
- **Async**: Use `tokio` runtime for all I/O
- **Serialization**: IPC messages use `serde` + `serde_json` over Unix domain sockets
- **Naming**: Snake_case for Rust, kebab-case for CLI arguments
- **No unsafe code** unless absolutely necessary and documented

## Testing

```bash
cargo test                    # Unit tests
cargo test --test integration # Integration tests
make test                     # All tests
```

Test categories:
- **Unit tests**: Individual agent validation logic
- **Integration tests**: End-to-end socket communication
- **Security tests**: Blocklist/allowlist boundary checks

## Adding a New Agent Type

1. Create `opagent-core/src/agents/<name>.rs`
2. Implement the `AgentExecutor` trait from `agents/mod.rs`
3. Add the variant to `Operation` enum in `security.rs`
4. Add validation logic in `Operation::security_level()`
5. Register in `daemon/src/main.rs` agent registry
6. Add CLI subcommand support in `opagentctl/src/main.rs`
7. Update `AgentConfig` in `config.rs` if the agent has config flags

## Tooling

- `make lint` — cargo clippy (all targets)
- `make format` — cargo fmt
- `make check` — cargo check (fast compile check)
- `journalctl -u opagentd` — view daemon logs
