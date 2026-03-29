import Foundation
import NostrSDK

/// Device linking protocol — allows Mac to receive identity from iPhone via Nostr relay
///
/// Flow:
/// 1. Mac generates ephemeral keypair + shows QR with sigil://link?npub=<ephemeral>&relay=<relay>
/// 2. iPhone scans QR → encrypts its nsec with ephemeral pubkey → sends via NIP-04 DM
/// 3. Mac receives → decrypts → saves key → now shares same identity as iPhone
///
/// Security:
/// - Ephemeral keypair is one-time (discarded after linking)
/// - nsec is NIP-04 encrypted in transit — relay can't read it
/// - Session auto-expires after 5 minutes

@MainActor
class DeviceLinkSession: ObservableObject {
    @Published var linkQRUri: String = ""
    @Published var isWaiting = false
    @Published var isLinked = false
    @Published var linkedNpub: String?
    @Published var error: String?

    private var ephemeralKeys: Keys?
    private var client: Client?
    private var signer: NostrSigner?
    private let relay: String

    init(relay: String = "wss://relay.damus.io") {
        self.relay = relay
    }

    /// Start a link session — generates QR for Mac to display
    func startSession() async {
        let keys = Keys.generate()
        self.ephemeralKeys = keys
        self.signer = NostrSigner.keys(keys: keys)

        let npub = (try? keys.publicKey().toBech32()) ?? ""
        self.linkQRUri = "sigil://link?npub=\(npub)&relay=\(relay.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? relay)"

        self.isWaiting = true
        self.isLinked = false
        self.error = nil

        // Connect and listen for the key delivery
        do {
            let signer = NostrSigner.keys(keys: keys)
            let client = ClientBuilder().signer(signer: signer).build()
            let relayUrl = try RelayUrl.parse(url: relay)
            _ = try await client.addRelay(url: relayUrl)
            await client.connect()
            self.client = client

            // Subscribe to DMs to this ephemeral key
            let filter = Filter()
                .kind(kind: Kind(kind: 4))
                .pubkey(pubkey: keys.publicKey())
            _ = try await client.subscribe(filter: filter, opts: nil)

            // Listen for the key delivery
            Task {
                let handler = LinkNotificationHandler { [weak self] event in
                    Task { @MainActor in
                        await self?.handleKeyDelivery(event)
                    }
                }
                try? await client.handleNotifications(handler: handler)
            }

            // Auto-expire after 5 minutes
            Task {
                try? await Task.sleep(for: .seconds(300))
                if !isLinked {
                    self.isWaiting = false
                    self.error = "Session expired. Try again."
                    await self.client?.disconnect()
                }
            }
        } catch {
            self.error = "Connection failed: \(error.localizedDescription)"
            self.isWaiting = false
        }
    }

    /// Handle incoming key delivery from iPhone
    private func handleKeyDelivery(_ event: Event) async {
        guard let signer = signer else { return }

        do {
            let content = try await signer.nip04Decrypt(
                publicKey: event.author(),
                encryptedContent: event.content()
            )

            // Content should be: sigil:key:<nsec>
            guard content.hasPrefix("sigil:key:") else { return }
            let nsec = String(content.dropFirst("sigil:key:".count))

            // Verify it's a valid key
            let newKeys = try Keys.parse(secretKey: nsec)
            let newNpub = try newKeys.publicKey().toBech32()

            // Save the key
            let keyFile = Self.keyFilePath()
            try nsec.write(to: keyFile, atomically: true, encoding: .utf8)

            self.linkedNpub = newNpub
            self.isLinked = true
            self.isWaiting = false

            // Disconnect ephemeral session
            await client?.disconnect()        } catch {
            self.error = "Failed to process key: \(error.localizedDescription)"
        }
    }

    /// Send our key to a link session (called by iPhone after scanning QR)
    static func sendKeyToSession(
        myKeys: Keys,
        ephemeralNpub: String,
        relay: String
    ) async throws {
        let ephemeralPubkey = try PublicKey.parse(publicKey: ephemeralNpub)
        let signer = NostrSigner.keys(keys: myKeys)
        let client = ClientBuilder().signer(signer: signer).build()
        let relayUrl = try RelayUrl.parse(url: relay)
        _ = try await client.addRelay(url: relayUrl)
        await client.connect()

        // Wait for connection
        try await Task.sleep(for: .seconds(2))

        // Send nsec encrypted with ephemeral pubkey
        let nsec = try myKeys.secretKey().toBech32()
        let content = "sigil:key:\(nsec)"
        let encrypted = try await signer.nip04Encrypt(
            publicKey: ephemeralPubkey,
            content: content
        )

        let tag = Tag.publicKey(publicKey: ephemeralPubkey)
        let builder = EventBuilder(kind: Kind(kind: 4), content: encrypted)
            .tags(tags: [tag])
        let event = try builder.signWithKeys(keys: myKeys)
        _ = try await client.sendEvent(event: event)

        await client.disconnect()    }

    /// Parse a sigil://link URI
    static func parseLinkUri(_ uri: String) -> (npub: String, relay: String)? {
        guard uri.hasPrefix("sigil://link?") else { return nil }
        let query = uri.replacingOccurrences(of: "sigil://link?", with: "")
        var npub: String?
        var relay: String?

        for pair in query.split(separator: "&") {
            let parts = pair.split(separator: "=", maxSplits: 1)
            guard parts.count == 2 else { continue }
            let key = String(parts[0])
            let value = String(parts[1]).removingPercentEncoding ?? String(parts[1])
            switch key {
            case "npub": npub = value
            case "relay": relay = value
            default: break
            }
        }

        guard let n = npub, let r = relay else { return nil }
        return (n, r)
    }

    private static func keyFilePath() -> URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Sigil", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("keys.nsec")
    }
}

/// Notification handler for link session
final class LinkNotificationHandler: HandleNotification, @unchecked Sendable {
    private let onEvent: @Sendable (Event) -> Void

    init(onEvent: @escaping @Sendable (Event) -> Void) {
        self.onEvent = onEvent
    }

    func handleMsg(relayUrl: RelayUrl, msg: RelayMessage) async {}

    func handle(relayUrl: RelayUrl, subscriptionId: String, event: Event) async {
        if event.kind().asU16() == 4 {
            onEvent(event)
        }
    }
}
