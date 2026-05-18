# opagentd

LLM-driven system agent daemon for Linux.

`opagentd` is a privileged background service that securely executes LLM-based AI agents on a Linux system. Every command — whether submitted as JSON or described in natural language — passes through a validation and approval pipeline before execution.

## Features

- **Natural language execution** — describe tasks in plain English; the LLM plans and executes them
- **Three agent types** — Shell, File, Git; extensible via trait
- **Security pipeline** — auto / confirm / deny, all checked before execution
- **LLM integration** — DeepSeek, OpenAI, Ollama, or any compatible provider
- **systemd service** — native Linux lifecycle management
- **CLI control** — full control via `opagentctl`
- **Audit logging** — structured JSON logs for every operation
- **Optional REST bridge** — `opagent-api` for HTTP access

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    opagentd (Daemon)                     │
│                                                         │
│  ┌──────────────┐    ┌───────────────┐    ┌───────────┐ │
│  │ LLM Engine   │───→│ Security      │───→│ Agent     │ │
│  │ (DeepSeek/   │    │ Pipeline      │    │ Executor  │ │
│  │  OpenAI/     │    │ auto/confirm/ │    │ Shell/    │ │
│  │  Ollama)     │    │ deny          │    │ File/Git  │ │
│  └──────────────┘    └───────────────┘    └───────────┘ │
│                                                         │
└─────────────────────┬───────────────────────────────────┘
                      │ /run/opagentd/opagentd.sock
                      │   (Unix Domain Socket)
┌─────────────────────┴───────────────────────────────────┐
│  opagentctl (CLI)                  opagent-api (REST)   │
│  submit / approve / deny /         HTTP POST endpoints  │
│  exec / validate / status /        127.0.0.1:9090       │
│  pending / logs                                         │
└─────────────────────────────────────────────────────────┘
```

### Security Model

```
User Input (JSON or natural language)
        │
        ▼
  ┌──────────────┐
  │   VALIDATE   │  Check against denied_commands, allowed_paths
  └──────┬───────┘
         │
    ┌────▼────┬─────────┬──────────┐
    │  auto   │ confirm │   deny   │
    │ execute │  queue  │  reject  │
    │  now    │         │          │
    └────┬────┘────┬────┘          │
         │         │               │
         ▼         ▼               ▼
    ┌─────────┐ ┌──────┐    ┌──────────┐
    │ Execute │ │Approve│   │ Rejected │
    │         │ │/Deny  │   │          │
    └────┬────┘ └───┬───┘   └──────────┘
         │          │
         ▼          ▼
    ┌────────────────────┐
    │   AUDIT LOG        │
    │ /var/log/opagentd/ │
    └────────────────────┘
```

## Requirements

- **Rust toolchain** (rustc 1.70+, cargo)
- **Linux** with systemd
- **git** (for Git Agent)
- **LLM API key** (for `exec` — one of DeepSeek, OpenAI, or Ollama)

## Quick Start

```bash
# Clone
git clone https://github.com/anomalyco/opagentd.git
cd opagentd

# Build
make build

# Install (requires root)
sudo make install

# Set your API key
sudo nano /etc/opagentd/config.toml
# → set [llm].api_key = "sk-..." or export DEEPSEEK_API_KEY

# Start
sudo systemctl daemon-reload
sudo systemctl start opagentd
sudo systemctl enable opagentd

# Verify
sudo opagentctl status
```

## Usage

> **Note:** If socket permissions are `0o660` (default), prefix commands with `sudo`.  
> To allow non-root usage, add your user to the `opagent` group and `chgrp` the socket, or set `permissions = 432` in config.

### Natural Language (LLM-Powered)

Describe tasks in plain English. The LLM translates intent into a multi-step plan of validated operations.

```bash
# System queries
opagentctl exec "show disk usage in human readable format"
opagentctl exec "who is currently logged in and what are they running"
opagentctl exec "list the top 5 largest files in /var/log"

# Multi-step tasks
opagentctl exec "create a file /tmp/system-report.txt with hostname, kernel version, and uptime"
# → 4 operations planned: hostname, uname -r, uptime -p, FileWrite

# Service management
opagentctl exec "restart nginx and check if it's healthy"

# Security — every LLM operation passes validation
opagentctl exec "read the shadow password file"
# → LLM plans FileRead /etc/shadow → denied (not in allowed_paths)

opagentctl exec "format the root partition"
# → LLM plans mkfs → denied by blocklist
```

### Validate Before Executing

Check an operation's security level without running it:

```bash
# Safe commands → auto
opagentctl validate '{"Shell":{"command":"ls","args":["-la"]}}'
opagentctl validate '{"Git":{"command":"status","repo_path":"/home/user/project"}}'

# Dangerous → deny
opagentctl validate '{"Shell":{"command":"rm","args":["-rf","/"]}}'
opagentctl validate '{"Shell":{"command":"dd","args":["if=/dev/zero","of=/dev/sda"]}}'

# Out-of-bounds path → deny
opagentctl validate '{"FileRead":{"path":"/etc/shadow"}}'

# Dangerous git → confirm (requires approval)
opagentctl validate '{"Git":{"command":"push --force origin main","repo_path":"/home/user/project"}}'
```

### Shell Agent

```bash
# Basic commands
opagentctl submit '{"Shell":{"command":"whoami","args":[]}}'
opagentctl submit '{"Shell":{"command":"date","args":["+%Y-%m-%d"]}}'
opagentctl submit '{"Shell":{"command":"df","args":["-h"]}}'

# With sudo-like subcommands
opagentctl submit '{"Shell":{"command":"systemctl","args":["restart","nginx"]}}'

# Shell pipe (use bash -c)
opagentctl submit '{"Shell":{"command":"bash","args":["-c","find /tmp -name '*.log' | wc -l"]}}'

# Build projects
opagentctl submit '{"Shell":{"command":"cargo","args":["build","--release"]}}'
```

### File Agent

```bash
# Read
opagentctl submit '{"FileRead":{"path":"/etc/hostname"}}'
opagentctl submit '{"FileRead":{"path":"/tmp/opagent/test.txt"}}'

# Write (creates parent directories automatically)
opagentctl submit '{"FileWrite":{"path":"/tmp/opagent/config.json","content":"{\"key\":\"value\"}"}}'

# Delete
opagentctl submit '{"FileDelete":{"path":"/tmp/opagent/config.json"}}'

# Full cycle: write → read → delete
opagentctl submit '{"FileWrite":{"path":"/tmp/demo.txt","content":"opagentd was here"}}'
opagentctl submit '{"FileRead":{"path":"/tmp/demo.txt"}}'
opagentctl submit '{"FileDelete":{"path":"/tmp/demo.txt"}}'
```

### Git Agent

```bash
# Status
opagentctl submit '{"Git":{"command":"status","repo_path":"/home/user/project"}}'

# History
opagentctl submit '{"Git":{"command":"log --oneline -5","repo_path":"/home/user/project"}}'

# Branching
opagentctl submit '{"Git":{"command":"checkout -b feature/example","repo_path":"/home/user/project"}}'

# Commit
opagentctl submit '{"Git":{"command":"commit -m \"agent update\"","repo_path":"/home/user/project"}}'

# Danger zone — requires approval
opagentctl submit '{"Git":{"command":"push --force origin main","repo_path":"/home/user/project"}}'
```

### Approval Workflow

When `default_level = "confirm"` in config, operations queue for human approval:

```bash
# 1. Submit — returns task_id
opagentctl submit '{"Shell":{"command":"whoami","args":[]}}'
# → {"submit":{"task_id":"task_18b0c20af7326927","level":"confirm","action":"Awaiting approval"}}

# 2. List pending
opagentctl pending
# → shows all queued tasks with descriptions and levels

# 3. Approve (executes immediately)
opagentctl approve task_18b0c20af7326927
# → {"executed":{"task_id":"...","success":true,"output":"root\n"}}

# 4. Or deny (discards without executing)
opagentctl deny task_18b0c20af7326927
# → {"denied":{"task_id":"..."}}
```

### Monitoring

```bash
# Daemon status
opagentctl status
# → {"status":{"running":true,"uptime_secs":3600,"tasks_pending":0}}

# Follow daemon logs
journalctl -u opagentd -f

# View audit log
opagentctl logs
opagentctl logs --count 50

# systemd health
systemctl status opagentd
```

## Configuration

Config file: `/etc/opagentd/config.toml`

```toml
[security]
# Default security level: "auto", "confirm", or "deny"
# - auto: execute safe commands immediately
# - confirm: queue for human approval (default)
# - deny: reject all by default
default_level = "confirm"

# Paths the File Agent is allowed to access
allowed_paths = ["/home", "/tmp", "/var/tmp"]

# Commands that are always rejected (substring match)
denied_commands = ["rm -rf /", "dd", "> /dev/sda", "mkfs"]

[socket]
path = "/run/opagentd/opagentd.sock"
# 0o660 = root:root rw only; 0o666 = world rw
permissions = 432

[logging]
level = "info"
audit_log = "/var/log/opagentd/audit.jsonl"

[agent]
shell_enabled = true
file_enabled = true
git_enabled = true

[llm]
# LLM provider: "deepseek", "openai", "ollama", or "custom"
provider = "deepseek"

# Model name (provider-specific)
model = "deepseek-chat"

# API base URL (optional; defaults by provider)
# base_url = "https://api.deepseek.com"

# API key — set here OR via environment variable:
#   DEEPSEEK_API_KEY for DeepSeek
#   OPENAI_API_KEY for OpenAI
#   OPAGENTD_API_KEY for custom provider
api_key = "sk-..."

# Generation parameters
temperature = 0.7
# max_tokens = 4096

# Toggle LLM integration
enabled = true
```

### Environment Variable Fallback

If `api_key` is not set in the config file, the daemon checks environment variables:

| Provider | Environment Variable |
|----------|---------------------|
| `deepseek` | `DEEPSEEK_API_KEY` |
| `openai` | `OPENAI_API_KEY` |
| `ollama` | (none needed) |
| `custom` | `OPAGENTD_API_KEY` |

## File System Layout

| Path | Purpose |
|------|---------|
| `/usr/local/bin/opagentd` | Daemon binary |
| `/usr/local/bin/opagentctl` | CLI client binary |
| `/etc/opagentd/config.toml` | Configuration |
| `/run/opagentd/opagentd.sock` | Runtime socket |
| `/var/log/opagentd/audit.jsonl` | Audit logs |

## Build from Source

```bash
git clone https://github.com/anomalyco/opagentd.git
cd opagentd

# Compile check
make check

# Run tests (18 tests)
make test

# Build release
make build

# Install
sudo make install
sudo systemctl daemon-reload
sudo systemctl enable --now opagentd
```

### Makefile Targets

| Target | Description |
|--------|-------------|
| `make check` | Fast compile check (no optimizations) |
| `make build` | Release build |
| `make test` | Run all tests |
| `make install` | Install binaries, systemd unit, config |
| `make uninstall` | Remove binaries and systemd unit |
| `make install-config` | Overwrite config with fresh defaults |
| `make lint` | Run clippy |
| `make format` | Run rustfmt |
| `make clean` | Remove build artifacts |

## Security Properties

- **Every operation is validated** before execution — no bypass
- **Path security** uses `canonicalize()` to resolve symlinks, preventing traversal
- **API keys never appear in logs** — `Debug` masks them as `***`
- **API keys never appear in serialized output** — `#[serde(skip_serializing)]`
- **Socket permissions** enforced at `bind()` time
- **Config is not overwritten** by `make install` — use `make install-config` to replace
- **No unsafe Rust**

## License

MIT
