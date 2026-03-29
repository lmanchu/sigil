import SwiftUI
#if canImport(UIKit)
import UIKit
#elseif canImport(AppKit)
import AppKit
#endif

struct AgentProfileView: View {
    @EnvironmentObject var nostrService: NostrService
    let agent: AgentContact
    @State private var editingName = false
    @State private var editName = ""
    @State private var editCodename = ""
    @State private var editAbout = ""

    var body: some View {
        List {
            // Header
            Section {
                VStack(spacing: 16) {
                    // Avatar
                    ZStack {
                        Circle()
                            .fill(
                                agent.isAgent
                                    ? SigilTheme.agentAccent.opacity(0.12)
                                    : SigilTheme.accent.opacity(0.1)
                            )
                            .frame(width: 88, height: 88)

                        Image(systemName: agent.isAgent ? "cpu" : "person.fill")
                            .font(.system(size: 36))
                            .foregroundStyle(agent.isAgent ? SigilTheme.agentAccent : SigilTheme.accent)
                    }

                    VStack(spacing: 6) {
                        HStack(spacing: 8) {
                            Text(agent.displayName)
                                .font(.title2)
                                .fontWeight(.bold)
                                .foregroundStyle(SigilTheme.adaptiveText)

                            if agent.isAgent {
                                Text("AGENT")
                                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 3)
                                    .background(SigilTheme.agentAccent.opacity(0.2))
                                    .foregroundStyle(SigilTheme.agentAccent)
                                    .clipShape(RoundedRectangle(cornerRadius: 4))
                            }
                        }

                        if agent.codename != nil && agent.codename != agent.name {
                            Text(agent.name)
                                .font(.caption)
                                .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                        }

                        if let about = agent.about {
                            Text(about)
                                .font(.subheadline)
                                .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                                .multilineTextAlignment(.center)
                        }

                        // Edit button
                        Button {
                            editName = agent.name
                            editCodename = agent.codename ?? ""
                            editAbout = agent.about ?? ""
                            editingName = true
                        } label: {
                            Label("Edit", systemImage: "pencil")
                                .font(.caption)
                                .foregroundStyle(SigilTheme.accent)
                        }
                    }
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
            }

            // Identity
            Section {
                VStack(alignment: .leading, spacing: 12) {
                    ProfileRow(icon: "key.fill", label: "npub", value: agent.shortNpub, mono: true)

                    if let relay = agent.relay {
                        ProfileRow(icon: "antenna.radiowaves.left.and.right", label: "Relay", value: relay, mono: true)
                    }

                    if let framework = agent.framework {
                        ProfileRow(icon: "hammer.fill", label: "Framework", value: framework)
                    }

                    ProfileRow(icon: "calendar", label: "Added", value: agent.addedAt.formatted(date: .abbreviated, time: .omitted))
                }
            } header: {
                Label("Identity", systemImage: "shield.fill")
                    .foregroundStyle(SigilTheme.accent)
            }

            // Capabilities
            if let caps = agent.capabilities, !caps.isEmpty {
                Section {
                    ForEach(caps, id: \.self) { cap in
                        HStack(spacing: 10) {
                            Image(systemName: "checkmark.seal.fill")
                                .font(.caption)
                                .foregroundStyle(SigilTheme.online)
                            Text(cap)
                                .font(.subheadline)
                                .foregroundStyle(SigilTheme.adaptiveText)
                        }
                    }
                } header: {
                    Label("Capabilities", systemImage: "cpu")
                        .foregroundStyle(SigilTheme.agentAccent)
                }
            }

            // QR Code — share this agent
            Section {
                VStack(spacing: 12) {
                    QRCodeView(uri: agent.inviteUri, label: "Scan to add this agent", size: 180)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 8)
            } header: {
                Label("Share Agent", systemImage: "qrcode")
                    .foregroundStyle(SigilTheme.accent)
            }

            // Actions
            Section {
                Button {
                    #if canImport(UIKit)
                    UIPasteboard.general.string = agent.npub
                    #elseif canImport(AppKit)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(agent.npub, forType: .string)
                    #endif
                } label: {
                    Label("Copy npub", systemImage: "doc.on.doc")
                        .foregroundStyle(SigilTheme.accent)
                }

                if agent.isAgent {
                    Button(role: .destructive) {
                        // TODO: remove agent
                    } label: {
                        Label("Remove Agent", systemImage: "trash")
                            .foregroundStyle(SigilTheme.danger)
                    }
                }
            }
        }
        .navigationTitle("Profile")
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
        .sheet(isPresented: $editingName) {
            NavigationStack {
                Form {
                    Section("Name") {
                        TextField("Name", text: $editName)
                    }
                    Section("Codename / Nickname") {
                        TextField("e.g. D.Gloria, Hermes", text: $editCodename)
                    }
                    Section("About") {
                        TextField("Description", text: $editAbout, axis: .vertical)
                            .lineLimit(3...6)
                    }
                }
                .navigationTitle("Edit Profile")
                #if os(iOS)
                .navigationBarTitleDisplayMode(.inline)
                #endif
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel") { editingName = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Save") {
                            agent.name = editName
                            agent.codename = editCodename.isEmpty ? nil : editCodename
                            agent.about = editAbout.isEmpty ? nil : editAbout
                            nostrService.saveContact(agent)
                            editingName = false
                        }
                        .fontWeight(.bold)
                    }
                }
            }
        }
    }
}

// MARK: - Profile Row

struct ProfileRow: View {
    let icon: String
    let label: String
    let value: String
    var mono: Bool = false

    var body: some View {
        HStack {
            Image(systemName: icon)
                .font(.caption)
                .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                .frame(width: 20)

            Text(label)
                .font(.subheadline)
                .foregroundStyle(SigilTheme.adaptiveTextSecondary)

            Spacer()

            Text(value)
                .font(mono ? .system(.caption, design: .monospaced) : .subheadline)
                .foregroundStyle(SigilTheme.adaptiveText)
        }
    }
}
