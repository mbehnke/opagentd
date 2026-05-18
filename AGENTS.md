# opagentd — Agents

## Übersicht

`opagentd` führt LLM-Agenten aus, die Systemoperationen autonom oder teil-autonom durchführen.

## Agent-Typen

| Agent | Beschreibung |
|-------|-------------|
| Shell Agent | Führt Shell-Befehle aus (validiert) |
| File Agent | Liest/schreibt Dateien im erlaubten Pfadbereich |
| Git Agent | Führt Git-Operationen aus |

## Sicherheitsstufen

1. **auto** – Vertrauenswürdige, vordefinierte Befehle
2. **confirm** – Benötigt Bestätigung via `opagentctl approve`
3. **deny** – Standard für unbekannte/riskante Operationen

## Konfiguration

`/etc/opagentd/config.toml`:

```toml
[security]
default_level = "confirm"
allowed_paths = ["/home", "/tmp"]
denied_commands = ["rm -rf /", "dd", "> /dev/sda"]
```
