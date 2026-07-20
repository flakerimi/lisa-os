# App intent manifest (MCP declaration)

Spec home for PLAN Appendix B; `lisa-agentd` (M5) validates and
registers these. An app declares its MCP capability in Flatpak metadata
/ a `.desktop` extension:

```json
{
  "lisa_manifest": 1,
  "app_id": "org.gnome.Calendar",
  "mcp": { "transport": "unix", "activatable": true },
  "tools": [{
    "name": "add_event",
    "tier": "write",
    "description": "Create a calendar event",
    "input_schema": {
      "type": "object",
      "required": ["title", "start"],
      "properties": {
        "title": { "type": "string" },
        "start": { "type": "string", "format": "date-time" },
        "end":   { "type": "string", "format": "date-time" }
      }
    },
    "undo": { "tool": "delete_event", "map": { "event_id": "$result.event_id" } }
  }],
  "resources": [
    { "uri": "selection://current", "description": "Currently selected event" }
  ]
}
```

## Field notes

- `tier` — confirmation tier enforced *at the bus*, not by app goodwill
  (PLAN §5.4): `read` (silent, ledgered), `write` (inline confirmation
  chip, batchable), `destructive` (explicit modal with typed diff).
- `undo` — compensation call recorded in the undo journal; `$result.*`
  maps the original call's result into the inverse call's arguments.
- `resources` — MCP resources; `selection://current` is the first-class
  selection-context primitive (PLAN §5.6).
- Transport is MCP over a per-app unix socket; `activatable: true`
  means the bus may spawn the app on demand (D-Bus activation
  semantics).

A JSON Schema for validation plus the registry format land with
`daemons/agentd` in M5.
