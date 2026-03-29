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

            // QR / Invite
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
