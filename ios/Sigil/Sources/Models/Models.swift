import Foundation

struct AgentContact: Identifiable, Equatable {
    let id = UUID()
    let npub: String
    var name: String
    var isAgent: Bool
    var relay: String?
    var lastSeen: Date?

    static func == (lhs: AgentContact, rhs: AgentContact) -> Bool {
        lhs.npub == rhs.npub
    }
}

struct ChatMessage: Identifiable {
    let id: String
    let content: String
    let senderNpub: String
    let isFromMe: Bool
    let timestamp: Date

    /// Check if this is a TUI message (JSON with type field)
    var isTui: Bool {
        content.trimmingCharacters(in: .whitespaces).hasPrefix("{")
            && content.contains("\"type\"")
    }

    /// Parse TUI message type
    var tuiType: String? {
        guard isTui,
              let data = content.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let type = json["type"] as? String
        else { return nil }
        return type
    }
}
