import Foundation
import SwiftData
import NostrSDK

/// Notification handler bridging nostr-sdk-swift to NostrService
final class NotificationHandler: HandleNotification, @unchecked Sendable {
    private let onEvent: @Sendable (RelayUrl, String, Event) -> Void

    init(onEvent: @escaping @Sendable (RelayUrl, String, Event) -> Void) {
        self.onEvent = onEvent
    }

    func handleMsg(relayUrl: RelayUrl, msg: RelayMessage) async {}

    func handle(relayUrl: RelayUrl, subscriptionId: String, event: Event) async {
        onEvent(relayUrl, subscriptionId, event)
    }
}

/// Core Nostr service — manages keys, relay connections, messaging, and persistence
@MainActor
class NostrService: ObservableObject {
    static let shared = NostrService()

    @Published var agents: [AgentContact] = []
    @Published var messages: [String: [ChatMessage]] = [:]
    @Published var isConnected = false

    private var client: Client?
    private var keys: Keys?
    private var modelContainer: ModelContainer?
    private var modelContext: ModelContext?

    private init() {
        loadOrCreateKeys()
        setupPersistence()
        loadFromStore()
    }

    // MARK: - Persistence

    private func setupPersistence() {
        do {
            let schema = Schema([AgentContact.self, ChatMessage.self])
            let config = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)
            let container = try ModelContainer(for: schema, configurations: [config])
            self.modelContainer = container
            self.modelContext = ModelContext(container)
        } catch {
            print("Persistence setup failed: \(error)")
        }
    }

    private func loadFromStore() {
        guard let context = modelContext else { return }

        // Load agents
        let agentDescriptor = FetchDescriptor<AgentContact>(sortBy: [SortDescriptor(\.addedAt)])
        if let stored = try? context.fetch(agentDescriptor) {
            self.agents = stored
        }

        // Load messages grouped by contact
        let msgDescriptor = FetchDescriptor<ChatMessage>(sortBy: [SortDescriptor(\.timestamp)])
        if let stored = try? context.fetch(msgDescriptor) {
            for msg in stored {
                let key = msg.isFromMe ? msg.recipientNpub : msg.senderNpub
                if messages[key] == nil { messages[key] = [] }
                messages[key]?.append(msg)
            }
        }
    }

    private func saveAgent(_ agent: AgentContact) {
        guard let context = modelContext else { return }
        context.insert(agent)
        try? context.save()
    }

    private func saveMessage(_ msg: ChatMessage) {
        guard let context = modelContext else { return }
        context.insert(msg)
        try? context.save()
    }

    // MARK: - Key Management

    var npub: String {
        (try? keys?.publicKey().toBech32()) ?? "unknown"
    }

    private func loadOrCreateKeys() {
        let keyFile = Self.keyFilePath()

        if FileManager.default.fileExists(atPath: keyFile.path),
           let data = try? String(contentsOf: keyFile, encoding: .utf8),
           let loaded = try? Keys.parse(secretKey: data.trimmingCharacters(in: .whitespacesAndNewlines)) {
            keys = loaded
        } else {
            let newKeys = Keys.generate()
            keys = newKeys
            if let sk = try? newKeys.secretKey().toBech32() {
                try? sk.write(to: keyFile, atomically: true, encoding: .utf8)
            }
        }
    }

    private static func keyFilePath() -> URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Sigil", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("keys.nsec")
    }

    // MARK: - Connection

    func connect() async {
        guard let keys = keys else { return }

        do {
            let signer = NostrSigner.keys(keys: keys)
            let client = ClientBuilder().signer(signer: signer).build()
            let relayUrl = try RelayUrl.parse(url: "wss://relay.damus.io")
            _ = try await client.addRelay(url: relayUrl)
            await client.connect()

            self.client = client
            self.isConnected = true

            let dmKind = Kind(kind: 4)
            let filter = Filter()
                .kind(kind: dmKind)
                .pubkey(pubkey: keys.publicKey())

            _ = try await client.subscribe(filter: filter, opts: nil)

            let handler = NotificationHandler { [weak self] relayUrl, subscriptionId, event in
                Task { @MainActor in
                    self?.handleIncomingEvent(event)
                }
            }
            try await client.handleNotifications(handler: handler)
        } catch {
            print("Connection error: \(error)")
        }
    }

    // MARK: - Message Handling

    private func handleIncomingEvent(_ event: Event) {
        guard let keys = keys else { return }

        let eventKind = event.kind().asU16()
        guard eventKind == 4 else { return }

        let eventId = event.id().toHex()

        // Dedup — skip if already stored
        if messages.values.flatMap({ $0 }).contains(where: { $0.messageId == eventId }) {
            return
        }

        do {
            let content = try nip04Decrypt(
                secretKey: keys.secretKey(),
                publicKey: event.author(),
                encryptedContent: event.content()
            )
            let senderNpub = (try? event.author().toBech32()) ?? "unknown"
            let msg = ChatMessage(
                messageId: eventId,
                content: content,
                senderNpub: senderNpub,
                recipientNpub: self.npub,
                isFromMe: false,
                timestamp: Date(timeIntervalSince1970: TimeInterval(event.createdAt().asSecs()))
            )

            if self.messages[senderNpub] == nil {
                self.messages[senderNpub] = []
            }
            self.messages[senderNpub]?.append(msg)
            saveMessage(msg)

            // Auto-add as agent contact if not already known
            if !self.agents.contains(where: { $0.npub == senderNpub }) {
                let agent = AgentContact(
                    npub: senderNpub,
                    name: "Agent \(senderNpub.prefix(12))...",
                    isAgent: true
                )
                self.agents.append(agent)
                saveAgent(agent)
            }
        } catch {
            print("Decrypt error: \(error)")
        }
    }

    // MARK: - Send Message

    func sendMessage(to npub: String, content: String) async {
        guard let client = client, let keys = keys else { return }

        do {
            let recipient = try PublicKey.parse(publicKey: npub)
            let encrypted = try nip04Encrypt(
                secretKey: keys.secretKey(),
                publicKey: recipient,
                content: content
            )

            let dmKind = Kind(kind: 4)
            let tag = Tag.publicKey(publicKey: recipient)
            let builder = EventBuilder(kind: dmKind, content: encrypted)
                .tags(tags: [tag])
            let event = try builder.signWithKeys(keys: keys)

            _ = try await client.sendEvent(event: event)

            let msg = ChatMessage(
                messageId: UUID().uuidString,
                content: content,
                senderNpub: self.npub,
                recipientNpub: npub,
                isFromMe: true,
                timestamp: Date()
            )

            if messages[npub] == nil {
                messages[npub] = []
            }
            messages[npub]?.append(msg)
            saveMessage(msg)
        } catch {
            print("Send error: \(error)")
        }
    }

    // MARK: - Contact Management

    func addAgent(_ agent: AgentContact) {
        if !agents.contains(where: { $0.npub == agent.npub }) {
            agents.append(agent)
            saveAgent(agent)
        }
    }

    // MARK: - QR Parsing

    func addAgentFromQR(_ uri: String) -> Bool {
        guard uri.hasPrefix("sigil://agent?") else { return false }

        let query = uri.replacingOccurrences(of: "sigil://agent?", with: "")
        var npub: String?
        var name: String?
        var relay: String?

        for pair in query.split(separator: "&") {
            let parts = pair.split(separator: "=", maxSplits: 1)
            guard parts.count == 2 else { continue }
            let key = String(parts[0])
            let value = String(parts[1])
                .replacingOccurrences(of: "%3A%2F%2F", with: "://")
                .replacingOccurrences(of: "%2F", with: "/")
                .replacingOccurrences(of: "%20", with: " ")

            switch key {
            case "npub": npub = value
            case "name": name = value
            case "relay": relay = value
            default: break
            }
        }

        guard let agentNpub = npub else { return false }

        let agent = AgentContact(
            npub: agentNpub,
            name: name ?? "Agent",
            isAgent: true,
            relay: relay
        )
        addAgent(agent)
        return true
    }
}
