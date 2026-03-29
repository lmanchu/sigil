# Agent Identity — See NIP-AE

This functionality is covered by **NIP-AE: Agents** in the upstream nostr-protocol/nips repo.

NIP-AE defines:
- **Kind 4199**: Agent Definition (identity, capabilities, tools)
- **Kind 4201**: Agent Nudge (behavioral modification)
- **Kind 4129**: Agent Lesson (learned behaviors)
- **Kind 14199**: Owner Claims (bidirectional verification)
- Agent Profile via kind:0 with `["bot"]` tag

Sigil's current implementation uses a simplified version of this:
- kind:0 with `"agent": true` custom field (should migrate to `["bot"]` tag)
- kind:31990 for registry (should evaluate alignment with kind:4199)

## Migration Plan

1. Add `["bot"]` tag to agent kind:0 events (NIP-AE standard)
2. Evaluate migrating registry from kind:31990 to kind:4199
3. Add `["p", "<owner-pubkey>"]` for owner-agent verification

See: https://github.com/nostr-protocol/nips/tree/agents
