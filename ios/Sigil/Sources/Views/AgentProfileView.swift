import SwiftUI
import UIKit

struct AgentProfileView: View {
    let agent: AgentContact

    var body: some View {
        List {
            // Avatar + Name
            Section {
                HStack(spacing: 16) {
                    ZStack {
                        Circle()
                            .fill(agent.isAgent ? .blue.opacity(0.15) : .green.opacity(0.15))
                            .frame(width: 72, height: 72)
                        Text(agent.isAgent ? "🤖" : "👤")
                            .font(.system(size: 36))
                    }

                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(agent.name)
                                .font(.title2)
                                .fontWeight(.bold)
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

                        if let about = agent.about {
                            Text(about)
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .padding(.vertical, 8)
            }

            // Identity
            Section("Identity") {
                LabeledContent("npub") {
                    Text(agent.shortNpub)
                        .font(.caption)
                        .monospaced()
                }

                if let relay = agent.relay {
                    LabeledContent("Relay") {
                        Text(relay)
                            .font(.caption)
                    }
                }

                if let framework = agent.framework {
                    LabeledContent("Framework") {
                        Text(framework)
                            .font(.caption)
                    }
                }

                LabeledContent("Added") {
                    Text(agent.addedAt, style: .date)
                }
            }

            // Capabilities
            if let caps = agent.capabilities, !caps.isEmpty {
                Section("Capabilities") {
                    ForEach(caps, id: \.self) { cap in
                        Label(cap, systemImage: "checkmark.circle.fill")
                            .foregroundStyle(.green)
                    }
                }
            }

            // Actions
            Section {
                Button {
                    UIPasteboard.general.string = agent.npub
                } label: {
                    Label("Copy npub", systemImage: "doc.on.doc")
                }

                if agent.isAgent {
                    Button(role: .destructive) {
                        // TODO: remove agent
                    } label: {
                        Label("Remove Agent", systemImage: "trash")
                    }
                }
            }
        }
        .navigationTitle("Profile")
        .navigationBarTitleDisplayMode(.inline)
    }
}
