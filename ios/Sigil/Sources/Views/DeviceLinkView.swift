import SwiftUI

/// Mac side — shows QR code for iPhone to scan
struct DeviceLinkQRView: View {
    @StateObject private var session = DeviceLinkSession()
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            if session.isLinked {
                // Success
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 60))
                    .foregroundStyle(SigilTheme.online)

                Text("Linked!")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundStyle(SigilTheme.adaptiveText)

                if let npub = session.linkedNpub {
                    Text(npub.prefix(20) + "...")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                }

                Text("Restart Sigil to use your linked identity.")
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)

                Button("Done") { dismiss() }
                    .font(.headline)
                    .padding(.horizontal, 40)
                    .padding(.vertical, 12)
                    .background(SigilTheme.accent)
                    .foregroundStyle(.black)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

            } else if session.isWaiting {
                // Waiting — show QR
                Image(systemName: "iphone.and.arrow.forward")
                    .font(.system(size: 40))
                    .foregroundStyle(SigilTheme.accent)

                Text("Link with iPhone")
                    .font(.title2)
                    .fontWeight(.bold)
                    .foregroundStyle(SigilTheme.adaptiveText)

                Text("Open Sigil on your iPhone and scan this QR code")
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 40)

                if !session.linkQRUri.isEmpty {
                    QRCodeView(uri: session.linkQRUri, label: "Scan with iPhone", size: 240)
                }

                ProgressView()
                    .tint(SigilTheme.accent)

                Text("Waiting for iPhone...")
                    .font(.caption)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)

            } else if let error = session.error {
                // Error
                Image(systemName: "exclamationmark.triangle")
                    .font(.system(size: 40))
                    .foregroundStyle(SigilTheme.warning)

                Text(error)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)

                Button("Try Again") {
                    Task { await session.startSession() }
                }
                .foregroundStyle(SigilTheme.accent)
            }

            Spacer()
        }
        .background(SigilTheme.adaptiveBg.ignoresSafeArea())
        .task {
            await session.startSession()
        }
    }
}

/// iPhone side — after scanning a sigil://link QR, confirms and sends key
struct DeviceLinkConfirmView: View {
    let linkNpub: String
    let linkRelay: String
    @EnvironmentObject var nostrService: NostrService
    @Environment(\.dismiss) private var dismiss
    @State private var isSending = false
    @State private var sent = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            if sent {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 60))
                    .foregroundStyle(SigilTheme.online)

                Text("Identity Sent!")
                    .font(.title2)
                    .fontWeight(.bold)

                Text("Your Mac should now show 'Linked'. Restart Sigil on Mac to complete.")
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 40)

                Button("Done") { dismiss() }
                    .font(.headline)
                    .padding(.horizontal, 40)
                    .padding(.vertical, 12)
                    .background(SigilTheme.accent)
                    .foregroundStyle(.black)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

            } else if let error {
                Image(systemName: "exclamationmark.triangle")
                    .font(.system(size: 40))
                    .foregroundStyle(SigilTheme.warning)
                Text(error)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                Button("Dismiss") { dismiss() }
                    .foregroundStyle(SigilTheme.accent)

            } else {
                Image(systemName: "desktopcomputer")
                    .font(.system(size: 50))
                    .foregroundStyle(SigilTheme.accent)

                Text("Link Mac?")
                    .font(.title2)
                    .fontWeight(.bold)

                Text("This will share your Sigil identity with the Mac. Both devices will use the same account.")
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 40)

                Text("Relay: \(linkRelay)")
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)

                if isSending {
                    ProgressView("Sending identity...")
                        .tint(SigilTheme.accent)
                } else {
                    Button {
                        sendKey()
                    } label: {
                        Text("Confirm Link")
                            .font(.headline)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 14)
                            .background(SigilTheme.accent)
                            .foregroundStyle(.black)
                            .clipShape(RoundedRectangle(cornerRadius: 14))
                    }
                    .padding(.horizontal, 40)

                    Button("Cancel") { dismiss() }
                        .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                }
            }

            Spacer()
        }
    }

    private func sendKey() {
        isSending = true
        Task {
            do {
                guard let keys = nostrService.keys else {
                    error = "No keys found on this device."
                    isSending = false
                    return
                }
                try await DeviceLinkSession.sendKeyToSession(
                    myKeys: keys,
                    ephemeralNpub: linkNpub,
                    relay: linkRelay
                )
                sent = true
            } catch {
                self.error = "Failed: \(error.localizedDescription)"
            }
            isSending = false
        }
    }
}
