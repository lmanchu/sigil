import Foundation
import SwiftData

// MARK: - User Profile

@Model
class UserProfile {
    @Attribute(.unique) var id: String // always "me"
    var displayName: String
    var about: String?
    var avatarData: Data? // locally stored photo
    var avatarUrl: String? // Nostr profile picture URL
    var createdAt: Date

    init() {
        self.id = "me"
        self.displayName = ""
        self.about = nil
        self.avatarData = nil
        self.avatarUrl = nil
        self.createdAt = Date()
    }
}

// MARK: - Agent / Contact

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
    var avatarData: Data? // locally assigned avatar for agents
    var capabilities: [String]?
    var framework: String?

    // Agent-specific
    var codename: String? // short codename like "D.Gloria"
    var agentVersion: String?

    var addedAt: Date
    var isFavorite: Bool

    init(npub: String, name: String, isAgent: Bool, relay: String? = nil) {
        self.npub = npub
        self.name = name
        self.isAgent = isAgent
        self.relay = relay
        self.addedAt = Date()
        self.isFavorite = false
    }

    /// Display name — codename first, then name
    var displayName: String {
        codename ?? name
    }

    /// Short display of npub
    var shortNpub: String {
        if npub.count > 16 {
            return "\(npub.prefix(8))...\(npub.suffix(4))"
        }
        return npub
    }

    /// Generate a sigil:// invite URI
    var inviteUri: String {
        let encodedRelay = (relay ?? "wss://relay.damus.io")
            .replacingOccurrences(of: "://", with: "%3A%2F%2F")
            .replacingOccurrences(of: "/", with: "%2F")
        let encodedName = displayName.replacingOccurrences(of: " ", with: "%20")
        return "sigil://agent?npub=\(npub)&relay=\(encodedRelay)&name=\(encodedName)"
    }
}

// MARK: - Chat Message

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

    var isTui: Bool {
        content.trimmingCharacters(in: .whitespaces).hasPrefix("{")
            && content.contains("\"type\"")
    }

    var tuiType: String? {
        guard isTui,
              let data = content.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let type = json["type"] as? String
        else { return nil }
        return type
    }
}
