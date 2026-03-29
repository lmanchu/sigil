import SwiftUI

#if canImport(UIKit) && !os(macOS)
import UIKit
import AVFoundation

struct QRScannerView: View {
    @EnvironmentObject var nostrService: NostrService
    @Environment(\.dismiss) private var dismiss
    @State private var scannedCode: String?
    @State private var showError = false
    @State private var showDeviceLink = false
    @State private var linkNpub = ""
    @State private var linkRelay = ""

    var body: some View {
        NavigationStack {
            ZStack {
                QRCameraView(scannedCode: $scannedCode)
                    .ignoresSafeArea()

                VStack {
                    Spacer()

                    VStack(spacing: 12) {
                        Image(systemName: "qrcode.viewfinder")
                            .font(.system(size: 40))
                            .foregroundStyle(.white)

                        Text("Scan Agent QR Code")
                            .font(.headline)
                            .foregroundStyle(.white)

                        Text("Point your camera at a Sigil agent QR code")
                            .font(.subheadline)
                            .foregroundStyle(.white.opacity(0.7))
                    }
                    .padding()
                    .background(.ultraThinMaterial)
                    .clipShape(RoundedRectangle(cornerRadius: 16))
                    .padding()
                }
            }
            .navigationTitle("Scan")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
            .onChange(of: scannedCode) { _, code in
                guard let code = code else { return }
                // Check if it's a device link QR
                if let linkData = DeviceLinkSession.parseLinkUri(code) {
                    linkNpub = linkData.npub
                    linkRelay = linkData.relay
                    showDeviceLink = true
                    scannedCode = nil
                } else if nostrService.addAgentFromQR(code) {
                    dismiss()
                } else {
                    showError = true
                    scannedCode = nil
                }
            }
            .sheet(isPresented: $showDeviceLink) {
                DeviceLinkConfirmView(linkNpub: linkNpub, linkRelay: linkRelay)
            }
            .alert("Invalid QR Code", isPresented: $showError) {
                Button("OK") {}
            } message: {
                Text("This QR code is not a Sigil agent. Look for QR codes with the sigil:// format.")
            }
        }
    }
}

// MARK: - Camera View (UIKit bridge)

struct QRCameraView: UIViewControllerRepresentable {
    @Binding var scannedCode: String?

    func makeUIViewController(context: Context) -> QRScannerController {
        let controller = QRScannerController()
        controller.delegate = context.coordinator
        return controller
    }

    func updateUIViewController(_ uiViewController: QRScannerController, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(scannedCode: $scannedCode)
    }

    class Coordinator: NSObject, QRScannerDelegate {
        @Binding var scannedCode: String?

        init(scannedCode: Binding<String?>) {
            _scannedCode = scannedCode
        }

        func didScanCode(_ code: String) {
            scannedCode = code
        }
    }
}

protocol QRScannerDelegate: AnyObject {
    func didScanCode(_ code: String)
}

class QRScannerController: UIViewController, @preconcurrency AVCaptureMetadataOutputObjectsDelegate {
    weak var delegate: QRScannerDelegate?
    private var captureSession: AVCaptureSession?
    private var hasScanned = false

    nonisolated override func viewDidLoad() {
        super.viewDidLoad()

        let session = AVCaptureSession()
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device) else { return }

        session.addInput(input)

        let output = AVCaptureMetadataOutput()
        session.addOutput(output)
        output.setMetadataObjectsDelegate(self, queue: .main)
        output.metadataObjectTypes = [.qr]

        let preview = AVCaptureVideoPreviewLayer(session: session)
        preview.frame = view.bounds
        preview.videoGravity = .resizeAspectFill
        view.layer.addSublayer(preview)

        captureSession = session
        DispatchQueue.global(qos: .userInitiated).async {
            session.startRunning()
        }
    }

    nonisolated func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !hasScanned,
              let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              let code = object.stringValue else { return }

        hasScanned = true
        captureSession?.stopRunning()
        delegate?.didScanCode(code)
    }
}

#else

// macOS fallback — paste URI instead of camera
struct QRScannerView: View {
    @EnvironmentObject var nostrService: NostrService
    @Environment(\.dismiss) private var dismiss
    @State private var uriInput = ""
    @State private var showError = false

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "qrcode.viewfinder")
                .font(.system(size: 60))
                .foregroundStyle(.secondary)

            Text("Add Agent")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Paste a sigil:// URI to add an agent")
                .foregroundStyle(.secondary)

            TextField("sigil://agent?npub=...", text: $uriInput)
                .textFieldStyle(.roundedBorder)
                .padding(.horizontal, 40)

            HStack(spacing: 16) {
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)

                Button("Add") {
                    if nostrService.addAgentFromQR(uriInput.trimmingCharacters(in: .whitespacesAndNewlines)) {
                        dismiss()
                    } else {
                        showError = true
                    }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(uriInput.isEmpty)
            }
        }
        .padding(40)
        .alert("Invalid URI", isPresented: $showError) {
            Button("OK") {}
        } message: {
            Text("Not a valid sigil:// agent URI.")
        }
    }
}

#endif
