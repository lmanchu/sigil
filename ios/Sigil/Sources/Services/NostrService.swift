import Foundation
import NostrSDK

/// Notification handler that bridges nostr-sdk-swift's HandleNotification protocol
/// to our @MainActor NostrService via a callback.
final class NotificationHandler: HandleNotification, @unchecked Sendable {
    private let onEvent: @Sendable (RelayUrl, String, Event) -> Void

    init(onEvent: @escaping @Sendable (RelayUrl, String, Event) -> Void) {
        self.onEvent = onEvent
    }

    func handleMsg(relayUrl: RelayUrl, msg: RelayMessage) async {
        // We only care about events, not raw relay messages
    }

    func handle(relayUrl: RelayUrl, subscriptionId: String, event: Event) async {
        onEvent(relayUrl, subscriptionId, event)
    }
}

/// Core Nostr service — manages keys, relay connections, and messaging
@MainActor
class NostrService: ObservableObject {
    static let shared = NostrService()

    @Published var agents: [AgentContact] = []
    @Published var messages: [String: [ChatMessage]] = [:] // keyed by npub
    @Published var isConnected = false

    private var client: Client?
    private var keys: Keys?

    private init() {
        loadOrCreateKeys()
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

            // Subscribe to DMs (NIP-04, kind 4)
            let dmKind = Kind(kind: 4)
            let filter = Filter()
                .kind(kind: dmKind)
                .pubkey(pubkey: keys.publicKey())

            _ = try await client.subscribe(filter: filter, opts: nil)

            // Start listening for incoming events
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
        guard eventKind == 4 else { return } // NIP-04 encrypted DM

        do {
            let content = try nip04Decrypt(
                secretKey: keys.secretKey(),
                publicKey: event.author(),
                encryptedContent: event.content()
            )
            let senderNpub = (try? event.author().toBech32()) ?? "unknown"
            let msg = ChatMessage(
                id: event.id().toHex(),
                content: content,
                senderNpub: senderNpub,
                isFromMe: false,
                timestamp: Date(timeIntervalSince1970: TimeInterval(event.createdAt().asSecs()))
            )

            if self.messages[senderNpub] == nil {
                self.messages[senderNpub] = []
            }
            self.messages[senderNpub]?.append(msg)

            // Auto-add as agent contact if not already known
            if !self.agents.contains(where: { $0.npub == senderNpub }) {
                self.agents.append(AgentContact(
                    npub: senderNpub,
                    name: "Agent \(senderNpub.prefix(12))...",
                    isAgent: true
                ))
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
                id: UUID().uuidString,
                content: content,
                senderNpub: self.npub,
                isFromMe: true,
                timestamp: Date()
            )

            if messages[npub] == nil {
                messages[npub] = []
            }
            messages[npub]?.append(msg)
        } catch {
            print("Send error: \(error)")
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

        if !agents.contains(where: { $0.npub == agentNpub }) {
            agents.append(AgentContact(
                npub: agentNpub,
                name: name ?? "Agent",
                isAgent: true,
                relay: relay
            ))
        }

        return true
    }
}
