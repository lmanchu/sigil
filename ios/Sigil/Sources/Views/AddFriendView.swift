import SwiftUI

struct AddFriendView: View {
    @EnvironmentObject var nostrService: NostrService
    @Environment(\.dismiss) private var dismiss
    @State private var searchText = ""
    @State private var friendName = ""
    @State private var isAgent = false
    @State private var codename = ""
    @State private var showError = false
    @State private var errorMessage = ""
    @State private var addedSuccessfully = false

    var body: some View {
        NavigationStack {
            List {
                // Search / Add by npub or URI
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Enter npub or sigil:// link")
                            .font(.caption)
                            .foregroundStyle(SigilTheme.adaptiveTextSecondary)

                        TextField("npub1... or sigil://agent?...", text: $searchText)
                            .font(.system(.body, design: .monospaced))
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()

                        if !searchText.isEmpty && !searchText.starts(with: "sigil://") {
                            TextField("Display name", text: $friendName)

                            Toggle("This is an agent", isOn: $isAgent)

                            if isAgent {
                                TextField("Codename (e.g. D.Gloria)", text: $codename)
                            }
                        }
                    }
                } header: {
                    Label("Add by ID", systemImage: "magnifyingglass")
                        .foregroundStyle(SigilTheme.accent)
                }

                // Quick Actions
                Section {
                    Button {
                        pasteFromClipboard()
                    } label: {
                        Label("Paste from Clipboard", systemImage: "doc.on.clipboard")
                            .foregroundStyle(SigilTheme.accent)
                    }

                    NavigationLink {
                        QRScannerView()
                    } label: {
                        Label("Scan QR Code", systemImage: "qrcode.viewfinder")
                            .foregroundStyle(SigilTheme.accent)
                    }
                } header: {
                    Label("Quick Add", systemImage: "bolt.fill")
                        .foregroundStyle(SigilTheme.accent)
                }

                // Share My Link
                Section {
                    Button {
                        shareMyLink()
                    } label: {
                        Label("Share My Sigil Link", systemImage: "square.and.arrow.up")
                            .foregroundStyle(SigilTheme.accent)
                    }

                    Button {
                        #if canImport(UIKit)
                        UIPasteboard.general.string = myInviteUri
                        #endif
                    } label: {
                        Label("Copy My Invite Link", systemImage: "doc.on.doc")
                            .foregroundStyle(SigilTheme.accent)
                    }
                } header: {
                    Label("Invite Friends", systemImage: "person.badge.plus")
                        .foregroundStyle(SigilTheme.accent)
                }

                // Test Agents
                Section {
                    Button {
                        addEchoAgent()
                    } label: {
                        HStack {
                            Image(systemName: "cpu")
                                .foregroundStyle(SigilTheme.agentAccent)
                            VStack(alignment: .leading) {
                                Text("Echo Agent")
                                    .foregroundStyle(SigilTheme.adaptiveText)
                                Text("Test agent — echoes your messages back")
                                    .font(.caption)
                                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                            }
                        }
                    }

                    Button {
                        addHermesBridge()
                    } label: {
                        HStack {
                            Image(systemName: "cpu")
                                .foregroundStyle(SigilTheme.agentAccent)
                            VStack(alignment: .leading) {
                                Text("Hermes Bridge")
                                    .foregroundStyle(SigilTheme.adaptiveText)
                                Text("155+ skills via encrypted DM")
                                    .font(.caption)
                                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                            }
                        }
                    }
                } header: {
                    Label("Test Agents", systemImage: "testtube.2")
                        .foregroundStyle(SigilTheme.agentAccent)
                }

                // Add Button
                if !searchText.isEmpty {
                    Section {
                        Button {
                            addContact()
                        } label: {
                            Text("Add Contact")
                                .font(.headline)
                                .frame(maxWidth: .infinity)
                                .foregroundStyle(.black)
                        }
                        .listRowBackground(SigilTheme.accent)
                    }
                }
            }
            .navigationTitle("Add Contact")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    Button("Done") { dismiss() }
                }
            }
            .alert("Error", isPresented: $showError) {
                Button("OK") {}
            } message: {
                Text(errorMessage)
            }
            .alert("Added!", isPresented: $addedSuccessfully) {
                Button("OK") { dismiss() }
            } message: {
                Text("Contact added successfully.")
            }
        }
    }

    private var myInviteUri: String {
        let npub = nostrService.npub
        let name = nostrService.userDisplayName
        return "sigil://user?npub=\(npub)&name=\(name.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? "")"
    }

    private func pasteFromClipboard() {
        #if canImport(UIKit)
        if let text = UIPasteboard.general.string {
            searchText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        #endif
    }

    private func addContact() {
        let text = searchText.trimmingCharacters(in: .whitespacesAndNewlines)

        if text.starts(with: "sigil://") {
            if nostrService.addAgentFromQR(text) {
                addedSuccessfully = true
            } else {
                errorMessage = "Invalid sigil:// URI or contact already exists."
                showError = true
            }
        } else if text.starts(with: "npub") {
            let name = friendName.isEmpty ? "Friend" : friendName
            let contact = AgentContact(npub: text, name: name, isAgent: isAgent)
            if isAgent && !codename.isEmpty {
                contact.codename = codename
            }
            nostrService.addAgent(contact)
            addedSuccessfully = true
        } else {
            errorMessage = "Enter a valid npub1... or sigil://... link."
            showError = true
        }
    }

    private func addEchoAgent() {
        let agent = AgentContact(
            npub: "npub13yuvfydn8g825p2w8nrp3a9vuh3ymc5cftyt433hzr3xzj7ppxms7jc060",
            name: "Echo Agent",
            isAgent: true,
            relay: "wss://relay.damus.io"
        )
        agent.codename = "Echo"
        agent.about = "Test agent — echoes your messages and demonstrates TUI components"
        nostrService.addAgent(agent)
        addedSuccessfully = true
    }

    private func addHermesBridge() {
        let agent = AgentContact(
            npub: "npub1052438peljxmmqq37ajpsgtc8rcdntvddvs8f6wctstzrckqp4jq3apu3u",
            name: "Hermes Bridge",
            isAgent: true,
            relay: "wss://relay.damus.io"
        )
        agent.codename = "Hermes"
        agent.about = "155+ OpenClaw & Hermes skills via encrypted Nostr DM"
        agent.capabilities = ["research", "github", "linear", "finance", "polymarket"]
        nostrService.addAgent(agent)
        addedSuccessfully = true
    }

    private func shareMyLink() {
        #if canImport(UIKit)
        let av = UIActivityViewController(activityItems: [myInviteUri], applicationActivities: nil)
        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
           let root = windowScene.windows.first?.rootViewController {
            root.present(av, animated: true)
        }
        #endif
    }
}
