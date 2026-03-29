import SwiftUI
import CoreImage.CIFilterBuiltins

/// Generates and displays a QR code from a sigil:// URI
struct QRCodeView: View {
    let uri: String
    let label: String
    var size: CGFloat = 220

    var body: some View {
        VStack(spacing: 16) {
            // QR Code
            if let image = generateQR(from: uri) {
                image
                    .interpolation(.none)
                    .resizable()
                    .scaledToFit()
                    .frame(width: size, height: size)
                    .background(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(SigilTheme.accent.opacity(0.3), lineWidth: 2)
                    )
            }

            Text(label)
                .font(.subheadline)
                .foregroundStyle(SigilTheme.adaptiveTextSecondary)
        }
    }

    private func generateQR(from string: String) -> Image? {
        let context = CIContext()
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)
        filter.correctionLevel = "M"

        guard let output = filter.outputImage else { return nil }

        // Scale up — CIFilter generates tiny images
        let scale = size / output.extent.size.width
        let scaled = output.transformed(by: CGAffineTransform(scaleX: scale, y: scale))

        guard let cgImage = context.createCGImage(scaled, from: scaled.extent) else { return nil }

        #if canImport(UIKit)
        return Image(uiImage: UIImage(cgImage: cgImage))
        #else
        return Image(nsImage: NSImage(cgImage: cgImage, size: NSSize(width: size, height: size)))
        #endif
    }
}

/// Full-screen QR display for showing to someone to scan
struct QRCardView: View {
    let uri: String
    let title: String
    let subtitle: String
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 28) {
            Spacer()

            // Title
            VStack(spacing: 8) {
                Text(title)
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundStyle(SigilTheme.adaptiveText)

                Text(subtitle)
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
            }

            // QR Code
            QRCodeView(uri: uri, label: "Scan with Sigil", size: 260)

            // Share button
            Button {
                shareUri()
            } label: {
                Label("Share Link", systemImage: "square.and.arrow.up")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
                    .background(SigilTheme.accent)
                    .foregroundStyle(.black)
                    .clipShape(RoundedRectangle(cornerRadius: 14))
            }
            .padding(.horizontal, 40)

            Spacer()

            Button("Done") { dismiss() }
                .foregroundStyle(SigilTheme.accent)
                .padding(.bottom, 20)
        }
        .background(SigilTheme.adaptiveBg.ignoresSafeArea())
    }

    private func shareUri() {
        #if canImport(UIKit)
        let av = UIActivityViewController(activityItems: [uri], applicationActivities: nil)
        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
           let root = windowScene.windows.first?.rootViewController {
            root.present(av, animated: true)
        }
        #endif
    }
}
