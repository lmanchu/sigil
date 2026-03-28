import SwiftUI

struct ContentView: View {
    @EnvironmentObject var nostrService: NostrService
    @State private var showQRScanner = false

    var body: some View {
        NavigationStack {
            Group {
                if nostrService.agents.isEmpty {
                    emptyState
                } else {
                    agentList
                }
            }
            .navigationTitle("Sigil")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        showQRScanner = true
                    } label: {
                        Image(systemName: "qrcode.viewfinder")
                    }
                }
                ToolbarItem(placement: .topBarLeading) {
                    Circle()
                        .fill(nostrService.isConnected ? .green : .red)
                        .frame(width: 8, height: 8)
                }
            }
            .sheet(isPresented: $showQRScanner) {
                QRScannerView()
            }
            .task {
                await nostrService.connect()
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 20) {
            Image(systemName: "bubble.left.and.bubble.right")
                .font(.system(size: 60))
                .foregroundStyle(.secondary)

            Text("No Agents Yet")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Scan an agent's QR code to start chatting")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Button {
                showQRScanner = true
            } label: {
                Label("Scan QR Code", systemImage: "qrcode.viewfinder")
                    .font(.headline)
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(.blue)
                    .foregroundStyle(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .padding(.horizontal, 40)
        }
    }

    private var agentList: some View {
        List(nostrService.agents) { agent in
            NavigationLink(destination: ChatView(agent: agent)) {
                HStack(spacing: 12) {
                    ZStack {
                        Circle()
                            .fill(.blue.opacity(0.15))
                            .frame(width: 44, height: 44)
                        Text(agent.isAgent ? "🤖" : "👤")
                            .font(.title2)
                    }

                    VStack(alignment: .leading, spacing: 2) {
                        HStack {
                            Text(agent.name)
                                .fontWeight(.medium)
                            if agent.isAgent {
                                Text("AGENT")
                                    .font(.caption2)
                                    .fontWeight(.bold)
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(.blue.opacity(0.15))
                                    .foregroundStyle(.blue)
                                    .clipShape(Capsule())
                            }
                        }

                        if let lastMsg = nostrService.messages[agent.npub]?.last {
                            Text(lastMsg.isTui ? "[Interactive Message]" : lastMsg.content)
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                        }
                    }

                    Spacer()
                }
                .padding(.vertical, 4)
            }
        }
    }
}
