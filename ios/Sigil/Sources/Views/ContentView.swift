import SwiftUI

struct ContentView: View {
    @EnvironmentObject var nostrService: NostrService
    @State private var showQRScanner = false
    @State private var showAddContact = false
    @State private var manualNpub = ""

    var body: some View {
        NavigationStack {
            Group {
                if nostrService.agents.isEmpty {
                    emptyState
                } else {
                    agentList
                }
            }
            .background(SigilTheme.adaptiveBg.ignoresSafeArea())
            .navigationTitle("Sigil")
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    HStack(spacing: 14) {
                        // Connection status
                        HStack(spacing: 6) {
                            Circle()
                                .fill(nostrService.isConnected ? SigilTheme.online : SigilTheme.danger)
                                .frame(width: 7, height: 7)
                            Text(nostrService.isConnected ? "Connected" : "Offline")
                                .font(.caption2)
                                .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                        }

                        Button { showAddContact = true } label: {
                            Image(systemName: "plus.circle")
                                .font(.title3)
                        }
                        Button { showQRScanner = true } label: {
                            Image(systemName: "qrcode.viewfinder")
                                .font(.title3)
                        }
                    }
                }
            }
            .sheet(isPresented: $showQRScanner) {
                QRScannerView()
            }
            .alert("Add Agent", isPresented: $showAddContact) {
                TextField("npub1...", text: $manualNpub)
                Button("Add") {
                    let npub = manualNpub.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !npub.isEmpty {
                        nostrService.addAgent(AgentContact(npub: npub, name: "Agent", isAgent: true))
                        manualNpub = ""
                    }
                }
                Button("Echo Agent") {
                    nostrService.addAgent(AgentContact(
                        npub: "npub13yuvfydn8g825p2w8nrp3a9vuh3ymc5cftyt433hzr3xzj7ppxms7jc060",
                        name: "Echo Agent",
                        isAgent: true
                    ))
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("Enter npub or add the echo agent for testing")
            }
            .task {
                await nostrService.connect()
            }
        }
    }

    // MARK: - Empty State

    private var emptyState: some View {
        VStack(spacing: 24) {
            Spacer()

            // Animated glyph
            ZStack {
                Circle()
                    .fill(SigilTheme.accent.opacity(0.08))
                    .frame(width: 120, height: 120)
                Circle()
                    .fill(SigilTheme.accent.opacity(0.15))
                    .frame(width: 80, height: 80)
                Image(systemName: "bubble.left.and.text.bubble.right")
                    .font(.system(size: 36))
                    .foregroundStyle(SigilTheme.accent)
            }

            VStack(spacing: 8) {
                Text("No Conversations")
                    .font(.title2)
                    .fontWeight(.bold)
                    .foregroundStyle(SigilTheme.adaptiveText)

                Text("Add an agent to start an encrypted conversation")
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 40)
            }

            VStack(spacing: 12) {
                Button { showQRScanner = true } label: {
                    Label("Scan QR Code", systemImage: "qrcode.viewfinder")
                        .font(.headline)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 14)
                        .background(SigilTheme.accent)
                        .foregroundStyle(.black)
                        .clipShape(RoundedRectangle(cornerRadius: 14))
                }

                Button { showAddContact = true } label: {
                    Label("Add Manually", systemImage: "plus")
                        .font(.headline)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 14)
                        .background(SigilTheme.adaptiveCard)
                        .foregroundStyle(SigilTheme.accent)
                        .clipShape(RoundedRectangle(cornerRadius: 14))
                        .overlay(
                            RoundedRectangle(cornerRadius: 14)
                                .stroke(SigilTheme.accent.opacity(0.3), lineWidth: 1)
                        )
                }
            }
            .padding(.horizontal, 40)

            Spacer()
            Spacer()
        }
    }

    // MARK: - Agent List

    private var agentList: some View {
        List(nostrService.agents) { agent in
            NavigationLink(destination: ChatView(agent: agent)) {
                AgentRow(agent: agent, lastMessage: nostrService.messages[agent.npub]?.last)
            }
            .listRowBackground(Color.clear)
            .listRowSeparatorTint(SigilTheme.adaptiveTextSecondary.opacity(0.15))
        }
        .listStyle(.plain)
    }
}

// MARK: - Agent Row

struct AgentRow: View {
    let agent: AgentContact
    let lastMessage: ChatMessage?

    var body: some View {
        HStack(spacing: 14) {
            // Avatar
            ZStack {
                Circle()
                    .fill(
                        agent.isAgent
                            ? SigilTheme.agentAccent.opacity(0.15)
                            : SigilTheme.accent.opacity(0.1)
                    )
                    .frame(width: 48, height: 48)

                if agent.isAgent {
                    Image(systemName: "cpu")
                        .font(.system(size: 20))
                        .foregroundStyle(SigilTheme.agentAccent)
                } else {
                    Image(systemName: "person.fill")
                        .font(.system(size: 20))
                        .foregroundStyle(SigilTheme.accent)
                }
            }

            // Name + last message
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text(agent.name)
                        .font(.body)
                        .fontWeight(.semibold)
                        .foregroundStyle(SigilTheme.adaptiveText)

                    if agent.isAgent {
                        Text("AGENT")
                            .font(.system(size: 9, weight: .bold, design: .monospaced))
                            .padding(.horizontal, 5)
                            .padding(.vertical, 2)
                            .background(SigilTheme.agentAccent.opacity(0.2))
                            .foregroundStyle(SigilTheme.agentAccent)
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                }

                if let msg = lastMessage {
                    Text(msg.isTui ? "Interactive message" : msg.content)
                        .font(.subheadline)
                        .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                        .lineLimit(1)
                }
            }

            Spacer()

            // Timestamp
            if let msg = lastMessage {
                Text(msg.timestamp, style: .time)
                    .font(.caption2)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
            }
        }
        .padding(.vertical, 6)
    }
}
