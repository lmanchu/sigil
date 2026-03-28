import Foundation
import SwiftData

@Model
class AgentContact {
    @Attribute(.unique) var npub: String
    var name: String
    var isAgent: Bool
    var relay: String?
    var lastSeen: Date?

    // Profile
    var about: String?
    var avatarUrl: String?
    var capabilities: [String]?
    var framework: String?

    var addedAt: Date

    init(npub: String, name: String, isAgent: Bool, relay: String? = nil) {
        self.npub = npub
        self.name = name
        self.isAgent = isAgent
        self.relay = relay
        self.addedAt = Date()
    }

    /// Short display of npub
    var shortNpub: String {
        if npub.count > 16 {
            return "\(npub.prefix(8))...\(npub.suffix(4))"
        }
        return npub
    }
}

@Model
class ChatMessage {
    @Attribute(.unique) var messageId: String
    var content: String
    var senderNpub: String
    var recipientNpub: String
    var isFromMe: Bool
    var timestamp: Date

    init(messageId: String, content: String, senderNpub: String, recipientNpub: String, isFromMe: Bool, timestamp: Date) {
        self.messageId = messageId
        self.content = content
        self.senderNpub = senderNpub
        self.recipientNpub = recipientNpub
        self.isFromMe = isFromMe
        self.timestamp = timestamp
    }

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
