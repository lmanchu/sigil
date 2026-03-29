import SwiftUI
import PhotosUI

struct MyProfileView: View {
    @EnvironmentObject var nostrService: NostrService
    @State private var displayName: String = ""
    @State private var about: String = ""
    @State private var selectedPhoto: PhotosPickerItem?
    @State private var avatarImage: Image?
    @State private var avatarData: Data?
    @State private var showCopied = false
    @State private var showQRCard = false
    @State private var iCloudSync = UserDefaults.standard.bool(forKey: "iCloudSyncEnabled")
    @State private var showRestartAlert = false

    var body: some View {
        List {
            // Avatar + Name
            Section {
                VStack(spacing: 16) {
                    // Avatar with photo picker
                    PhotosPicker(selection: $selectedPhoto, matching: .images) {
                        ZStack {
                            if let avatarImage {
                                avatarImage
                                    .resizable()
                                    .scaledToFill()
                                    .frame(width: 100, height: 100)
                                    .clipShape(Circle())
                            } else {
                                Circle()
                                    .fill(SigilTheme.accent.opacity(0.1))
                                    .frame(width: 100, height: 100)
                                Image(systemName: "person.fill")
                                    .font(.system(size: 40))
                                    .foregroundStyle(SigilTheme.accent)
                            }

                            // Camera badge
                            Circle()
                                .fill(SigilTheme.accent)
                                .frame(width: 28, height: 28)
                                .overlay(
                                    Image(systemName: "camera.fill")
                                        .font(.system(size: 12))
                                        .foregroundStyle(.black)
                                )
                                .offset(x: 36, y: 36)
                        }
                    }
                    .onChange(of: selectedPhoto) { _, item in
                        Task {
                            if let data = try? await item?.loadTransferable(type: Data.self) {
                                avatarData = data
                                #if canImport(UIKit)
                                if let uiImage = UIImage(data: data) {
                                    avatarImage = Image(uiImage: uiImage)
                                }
                                #endif
                            }
                        }
                    }

                    // Display name
                    TextField("Display Name", text: $displayName)
                        .font(.title2)
                        .fontWeight(.bold)
                        .multilineTextAlignment(.center)
                        .foregroundStyle(SigilTheme.adaptiveText)

                    // About
                    TextField("About (optional)", text: $about)
                        .font(.subheadline)
                        .multilineTextAlignment(.center)
                        .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
            }

            // My QR Code
            Section {
                VStack(spacing: 16) {
                    QRCodeView(uri: myInviteUri, label: "Scan to add me on Sigil", size: 200)

                    Button {
                        showQRCard = true
                    } label: {
                        Label("Show Full Screen", systemImage: "arrow.up.left.and.arrow.down.right")
                            .font(.subheadline)
                            .foregroundStyle(SigilTheme.accent)
                    }
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 8)
            } header: {
                Label("My QR Code", systemImage: "qrcode")
                    .foregroundStyle(SigilTheme.accent)
            }

            // Identity
            Section {
                VStack(alignment: .leading, spacing: 12) {
                    ProfileRow(icon: "key.fill", label: "npub", value: shortNpub, mono: true)

                    Button {
                        #if canImport(UIKit)
                        UIPasteboard.general.string = nostrService.npub
                        #endif
                        showCopied = true
                        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                            showCopied = false
                        }
                    } label: {
                        HStack {
                            Image(systemName: showCopied ? "checkmark" : "doc.on.doc")
                                .font(.caption)
                            Text(showCopied ? "Copied!" : "Copy Full npub")
                                .font(.subheadline)
                        }
                        .foregroundStyle(SigilTheme.accent)
                    }
                }
            } header: {
                Label("Identity", systemImage: "shield.fill")
                    .foregroundStyle(SigilTheme.accent)
            }

            // Invite
            Section {
                Button {
                    shareInvite()
                } label: {
                    Label("Share My Sigil Link", systemImage: "square.and.arrow.up")
                        .foregroundStyle(SigilTheme.accent)
                }
            } header: {
                Label("Invite Friends", systemImage: "person.badge.plus")
                    .foregroundStyle(SigilTheme.accent)
            }

            // iCloud Sync
            Section {
                Toggle(isOn: $iCloudSync) {
                    Label("iCloud Sync", systemImage: "icloud")
                }
                .tint(SigilTheme.accent)
                .onChange(of: iCloudSync) { _, newValue in
                    UserDefaults.standard.set(newValue, forKey: "iCloudSyncEnabled")
                    showRestartAlert = true
                }
            } header: {
                Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                    .foregroundStyle(SigilTheme.accent)
            } footer: {
                Text("Syncs contacts, messages, and profile across your devices. Keys are included — only enable if you trust your iCloud account. Requires app restart.")
                    .font(.caption)
            }

            // Save
            Section {
                Button {
                    nostrService.updateProfile(
                        displayName: displayName,
                        about: about.isEmpty ? nil : about,
                        avatarData: avatarData
                    )
                } label: {
                    Text("Save Profile")
                        .font(.headline)
                        .frame(maxWidth: .infinity)
                        .foregroundStyle(.black)
                }
                .listRowBackground(SigilTheme.accent)
            }
        }
        .navigationTitle("My Profile")
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
        .sheet(isPresented: $showQRCard) {
            QRCardView(
                uri: myInviteUri,
                title: displayName.isEmpty ? "Sigil User" : displayName,
                subtitle: "Scan to add me on Sigil"
            )
        }
        .alert("Restart Required", isPresented: $showRestartAlert) {
            Button("OK") {}
        } message: {
            Text("Please close and reopen Sigil for iCloud sync changes to take effect.")
        }
        .onAppear {
            displayName = nostrService.userDisplayName
            about = nostrService.userAbout ?? ""
            if let data = nostrService.userAvatarData {
                avatarData = data
                #if canImport(UIKit)
                if let uiImage = UIImage(data: data) {
                    avatarImage = Image(uiImage: uiImage)
                }
                #endif
            }
        }
    }

    private var myInviteUri: String {
        let name = displayName.isEmpty ? "Sigil User" : displayName
        return "sigil://user?npub=\(nostrService.npub)&name=\(name.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? "")"
    }

    private var shortNpub: String {
        let npub = nostrService.npub
        if npub.count > 20 {
            return "\(npub.prefix(10))...\(npub.suffix(6))"
        }
        return npub
    }

    private func shareInvite() {
        let uri = "sigil://user?npub=\(nostrService.npub)&name=\(displayName.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? "")"
        #if canImport(UIKit)
        let av = UIActivityViewController(activityItems: [uri], applicationActivities: nil)
        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
           let root = windowScene.windows.first?.rootViewController {
            root.present(av, animated: true)
        }
        #endif
    }
}
