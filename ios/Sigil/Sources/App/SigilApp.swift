import SwiftUI

@main
struct SigilApp: App {
    @StateObject private var nostrService = NostrService.shared

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(nostrService)
        }
    }
}
