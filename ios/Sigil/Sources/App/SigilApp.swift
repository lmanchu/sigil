import SwiftUI
import SwiftData

@main
struct SigilApp: App {
    @StateObject private var nostrService = NostrService.shared

    var sharedModelContainer: ModelContainer = {
        let schema = Schema([AgentContact.self, ChatMessage.self])
        let config = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)
        do {
            return try ModelContainer(for: schema, configurations: [config])
        } catch {
            fatalError("Could not create ModelContainer: \(error)")
        }
    }()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(nostrService)
        }
        .modelContainer(sharedModelContainer)
    }
}
