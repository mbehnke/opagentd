# opagentd

LLM-gesteuerter System-Agent-Daemon für Linux.

`opagentd` ist ein privilegierter Hintergrunddienst, der KI-Agenten (LLM-basiert) sicher auf einem Linux-System ausführt. Jeder Befehl durchläuft eine Validierungs- und Genehmigungspipeline, bevor er ausgeführt wird.

## Features

- LLM-gesteuerte Systemoperationen
- Sicherheits-Pipeline mit Genehmigungs-Workflow
- Systemd-Integration
- CLI-Steuerung via `opagentctl`

## Architektur

```
opagentd (Daemon)  ←→  LLM-Engine  ←→  Security Guard
     ↓
opagentctl (CLI)
```

## Installation

```bash
# Build & install
make && sudo make install

# Service starten
sudo systemctl enable --now opagentd
```

## Verwendung

```bash
opagentctl status
opagentctl exec "Befehl als natürliche Sprache"
opagentctl approve <task-id>
```

## Lizenz

MIT
