# NIP-YY: TUI Message Format

`draft` `optional`

## Abstract

This NIP defines a structured message format for AI agents to send interactive UI elements (buttons, cards, tables) through Nostr DMs. These messages are rendered by compatible clients as rich TUI/GUI components.

## Motivation

Plain text limits what agents can communicate. Agents need to present:

- Actionable buttons (confirm/cancel, menu options)
- Information cards (search results, summaries)
- Data tables (portfolio, stats, schedules)

Without a standard format, each agent-client pair must invent its own rendering protocol.

## Specification

### Message Format

TUI messages are JSON objects embedded in the content field of NIP-04 or NIP-17 encrypted DMs. They are distinguished from plain text by the presence of a `"type"` field.

### Message Types

#### Text

```json
{
  "type": "text",
  "content": "Hello! How can I help you today?"
}
```

#### Buttons

```json
{
  "type": "buttons",
  "text": "What would you like to do?",
  "items": [
    {"id": "weather", "label": "Check Weather", "style": "primary"},
    {"id": "news", "label": "Read News", "style": "secondary"},
    {"id": "cancel", "label": "Cancel", "style": "danger"}
  ]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | string | NO | Prompt text above buttons |
| `items` | Button[] | YES | List of buttons |
| `items[].id` | string | YES | Callback identifier |
| `items[].label` | string | YES | Display text |
| `items[].style` | string | NO | `primary`, `secondary`, or `danger` |

#### Card

```json
{
  "type": "card",
  "title": "Tokyo Weather",
  "description": "Clear skies, 22°C. High: 25°C, Low: 18°C.",
  "image_url": "https://example.com/weather-icon.png",
  "actions": [
    {"id": "forecast", "label": "5-Day Forecast", "style": "primary"},
    {"id": "alerts", "label": "Set Alert"}
  ]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | YES | Card title |
| `description` | string | NO | Card body text |
| `image_url` | string | NO | Image URL |
| `actions` | Button[] | NO | Action buttons |

#### Table

```json
{
  "type": "table",
  "title": "Portfolio Summary",
  "rows": [
    ["Asset", "Value"],
    ["BTC", "$67,000"],
    ["ETH", "$3,400"],
    ["Total", "$70,400"]
  ]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | NO | Table title |
| `rows` | [string, string][] | YES | Key-value rows |

### Button Callbacks

When a user taps a button, the client sends a callback message:

```
sigil:callback:<button_id>
```

For example, tapping the "Check Weather" button sends:

```
sigil:callback:weather
```

Agents SHOULD handle unknown callback IDs gracefully.

### Detection

Clients detect TUI messages by checking:
1. Content starts with `{`
2. Content parses as valid JSON
3. JSON contains a `"type"` field matching one of: `text`, `buttons`, `card`, `table`

If parsing fails, the content SHOULD be displayed as plain text.

### Fallback Rendering

Clients that don't support TUI messages SHOULD display:
- **text**: The `content` field as plain text
- **buttons**: The `text` field followed by numbered options
- **card**: Title + description as text
- **table**: Rows formatted as `key: value` lines

## Security Considerations

1. **Image URLs**: Clients SHOULD sanitize `image_url` values. Only HTTPS URLs should be loaded. Clients MAY proxy images to prevent IP leaks.

2. **Callback injection**: Button IDs are agent-defined strings. Clients MUST NOT execute button IDs as code. They are opaque identifiers sent back to the agent.

3. **Content size**: Clients SHOULD reject TUI messages larger than 64 KB to prevent abuse.

## Reference Implementation

- [sigil-core](https://github.com/lmanchu/sigil) — `sigil_core::tui::TuiMessage`
- Rendering: `sigil-cli` uses Ratatui for terminal rendering
- iOS: `SigilApp` renders TUI messages as native SwiftUI components
