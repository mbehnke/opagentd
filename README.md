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
opagentctl --help

# Start daemon
sudo systemctl start opagentd

# Enable at boot
sudo systemctl enable opagentd
```

## Usage

> **Note:** If socket permissions are `0o660` (default), use `sudo opagentctl`.  
> To allow non-root usage, add your user to the `opagent` group and chgrp the socket, or set `permissions = 432` in config.

### Check if the daemon is running

```bash
opagentctl status
# {"status":{"running":true,"uptime_secs":42,"tasks_pending":0}}
```

### Validate before executing

Validate an operation to see its security level without executing:

```bash
# Safe shell command → auto
opagentctl validate '{"Shell":{"command":"ls","args":["-la"]}}'

# Dangerous command → deny (blocked by denied_commands)
opagentctl validate '{"Shell":{"command":"rm","args":["-rf","/"]}}'

# Allowed file path
opagentctl validate '{"FileRead":{"path":"/home/user/document.txt"}}'

# Out-of-bounds file path → deny (not in allowed_paths)
opagentctl validate '{"FileRead":{"path":"/etc/shadow"}}'

# Safe git command → auto
opagentctl validate '{"Git":{"command":"status","repo_path":"/home/user/project"}}'

# Dangerous git command → confirm (requires approval)
opagentctl validate '{"Git":{"command":"push --force origin main","repo_path":"/home/user/project"}}'
```

### Submit operations for execution

#### Shell Agent

```bash
# Run a command (auto-executes if default_level = "auto")
opagentctl submit '{"Shell":{"command":"whoami","args":[]}}'
# → {"executed":{"task_id":"task_...","success":true,"output":"root\n"}}

# Run with arguments
opagentctl submit '{"Shell":{"command":"date","args":["+%Y-%m-%d"]}}'
# → {"executed":{"task_id":"task_...","success":true,"output":"2026-05-18\n"}}

# Run in specific directory
opagentctl submit '{"Shell":{"command":"pwd","args":[]}}'

# Build a project
opagentctl submit '{"Shell":{"command":"cargo","args":["build","--release"]}}'

# Restart a service
opagentctl submit '{"Shell":{"command":"systemctl","args":["restart","nginx"]}}'
```

#### File Agent

```bash
# Read a file
opagentctl submit '{"FileRead":{"path":"/etc/hostname"}}'

# Write a file (creates parent directories automatically)
opagentctl submit '{"FileWrite":{"path":"/tmp/opagent/config.json","content":"{\"key\":\"value\"}"}}'

# Delete a file
opagentctl submit '{"FileDelete":{"path":"/tmp/opagent/config.json"}}'

# Read project source
opagentctl submit '{"FileRead":{"path":"/home/user/project/Cargo.toml"}}'
```

#### Git Agent

```bash
# Check status
opagentctl submit '{"Git":{"command":"status","repo_path":"/home/user/project"}}'

# View recent commits
opagentctl submit '{"Git":{"command":"log --oneline -5","repo_path":"/home/user/project"}}'

# Pull latest changes (auto-executes)
opagentctl submit '{"Git":{"command":"pull --rebase","repo_path":"/home/user/project"}}'

# Create and switch branch
opagentctl submit '{"Git":{"command":"checkout -b feature/new-thing","repo_path":"/home/user/project"}}'

# Force-push (requires approval — confirm level)
opagentctl submit '{"Git":{"command":"push --force origin main","repo_path":"/home/user/project"}}'
```

### Approval workflow (when `default_level = "confirm"`)

```bash
# 1. Submit a task — returns task_id, queued for approval
opagentctl submit '{"Shell":{"command":"whoami","args":[]}}'
# → {"submit":{"task_id":"task_18b0c20af7326927","level":"confirm","action":"Awaiting approval"}}

# 2. List all pending tasks
opagentctl pending

# 3. Approve the task (executes immediately)
opagentctl approve task_18b0c20af7326927
# → {"executed":{"task_id":"task_18b0c20af7326927","success":true,"output":"root\n"}}

# 4. Or deny it (never executes)
opagentctl deny task_18b0c20af7326927
# → {"denied":{"task_id":"task_18b0c20af7326927"}}
```

### Full workflow example (auto mode)

With `default_level = "auto"` in config, safe commands execute immediately:

```bash
# File: write → read → delete — no approvals needed
opagentctl submit '{"FileWrite":{"path":"/tmp/demo.txt","content":"opagentd was here"}}'
opagentctl submit '{"FileRead":{"path":"/tmp/demo.txt"}}'
opagentctl submit '{"FileDelete":{"path":"/tmp/demo.txt"}}'

# Shell: all three in one
opagentctl submit '{"Shell":{"command":"bash","args":["-c","echo hello && ls /tmp && whoami"]}}'

# Git: commit + push
opagentctl submit '{"Git":{"command":"commit --allow-empty -m \"agent commit\"","repo_path":"/home/user/project"}}'
```

### LLM-powered execution (`opagentctl exec`)

Instead of crafting JSON operations manually, describe tasks in natural language.  
The LLM (DeepSeek, OpenAI, Ollama) translates your intent into validated operations.

```bash
# Simple commands
opagentctl exec "print the current date and time"
# → {"exec":{"reasoning":"1 operation(s) planned by LLM",...}}

# System queries
opagentctl exec "show disk usage in human readable format"
# → planned: "df -h" → executed

# Multi-step tasks (LLM plans, security validates each step)
opagentctl exec "who is currently logged in and what processes are they running"
# → planned: who, ps → 2 operations, auto-executed

# File operations
opagentctl exec "create a file /tmp/report.txt with system uptime info"
# → planned: 4 operations (hostname, uname, uptime, FileWrite) → executed

# Security example — the LLM plan passes through validation:
opagentctl exec "read the shadow password file"
# → LLM plans: FileRead /etc/shadow → deny (not in allowed_paths)

# Production example
opagentctl exec "restart nginx and check if it's healthy"
```

> **Security note:** Every LLM-generated operation passes through the same  
> validate → auto/confirm/deny pipeline as manual JSON operations.  
> Dangerous commands are blocked regardless of what the LLM generates.

### Monitor and debug

```bash
# View daemon logs in real time
journalctl -u opagentd -f

# View audit log
opagentctl logs

# View last 50 audit entries
opagentctl logs --count 50

# Check daemon health
systemctl status opagentd
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

[llm]
provider = "deepseek"
model = "deepseek-chat"
# api_key = "sk-..."          # or env var: DEEPSEEK_API_KEY
temperature = 0.7
enabled = true
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
