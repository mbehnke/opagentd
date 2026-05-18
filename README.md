# opagentd

LLM-driven system agent daemon for Linux.

`opagentd` is a privileged background service that securely executes LLM-based AI agents on a Linux system. Every command passes through a validation and approval pipeline before execution.

## Features

- **LLM-driven system operations** — describe tasks in natural language
- **Security pipeline** — validate → approve → execute, with three security levels
- **systemd integration** — native Linux service management
- **CLI control** — full control via `opagentctl`
- **Audit logging** — structured JSON logs for every operation
- **Extensible agents** — Shell, File, Git agents; add your own

## Architecture

```
opagentd (Daemon)  ←→  LLM-Engine  ←→  Security Guard
     ↑ ↓
/run/opagentd/opagentd.sock   (Unix Domain Socket)
     ↑ ↓
opagentctl (CLI)
```

```
Submit → Validate → SecurityLevel{auto,confirm,deny} → Execute → Audit
```

## Requirements

- Rust toolchain (rustc 1.70+, cargo)
- Linux with systemd
- git (for Git Agent)

## Quick Start

```bash
# Build
make build

# Install (requires root)
sudo make install

# Verify
opagentctl status

# Start daemon
sudo systemctl start opagentd

# Enable at boot
sudo systemctl enable opagentd
```

## Usage

```bash
# Check daemon status
opagentctl status

# Submit a task (natural language or structured)
opagentctl submit '{"type":"Shell","command":"whoami","args":[]}'

# List pending tasks
opagentctl pending

# Approve a task
opagentctl approve <task-id>

# Deny a task
opagentctl deny <task-id>

# Validate an operation without executing
opagentctl validate '{"type":"Shell","command":"rm","args":["-rf","/"]}'

# View audit logs
opagentctl logs

# View daemon logs
journalctl -u opagentd -f
```

## Configuration

See `/etc/opagentd/config.toml` after installation. Default config ships with:

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
```

## File System Layout

| Path | Purpose |
|------|---------|
| `/usr/local/bin/opagentd` | Daemon binary |
| `/usr/local/bin/opagentctl` | CLI binary |
| `/etc/opagentd/config.toml` | Configuration |
| `/run/opagentd/opagentd.sock` | Runtime socket |
| `/var/log/opagentd/audit.jsonl` | Audit log |

## Build from Source

```bash
git clone https://github.com/anomalyco/opagentd.git
cd opagentd
make build
sudo make install
```

## License

MIT
