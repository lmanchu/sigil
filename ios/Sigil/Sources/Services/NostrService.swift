import Foundation
import NostrSDK

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
        keys?.publicKey().toBech32() ?? "unknown"
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
            let client = Client(signer: .keys(keys: keys))
            try await client.addRelay(url: "wss://relay.damus.io")
            await client.connect()

            self.client = client
            self.isConnected = true

            // Subscribe to DMs (NIP-04)
            let filter = Filter()
                .kind(kind: .encryptedDirectMessage)
                .pubkey(publicKey: keys.publicKey())

            try await client.subscribe(filters: [filter])

            // Start listening
            await listenForMessages()
        } catch {
            print("Connection error: \(error)")
        }
    }

    // MARK: - Message Listening

    private func listenForMessages() async {
        guard let client = client, let keys = keys else { return }

        for await notification in client.notifications() {
            switch notification {
            case .event(let relayUrl, let event):
                if event.kind() == .encryptedDirectMessage {
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

                        await MainActor.run {
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
                        }
                    } catch {
                        print("Decrypt error: \(error)")
                    }
                }
            case .shutdown:
                await MainActor.run { self.isConnected = false }
            default:
                break
            }
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

            let tag = Tag.publicKey(publicKey: recipient)
            let event = try EventBuilder(kind: .encryptedDirectMessage, content: encrypted)
                .tag(tag: tag)
                .sign(keys: keys)

            try await client.sendEvent(event: event)

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
