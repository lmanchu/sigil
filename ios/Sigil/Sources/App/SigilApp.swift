import SwiftUI
import SwiftData

@main
struct SigilApp: App {
    @StateObject private var nostrService = NostrService.shared

    var sharedModelContainer: ModelContainer = {
        let schema = Schema([UserProfile.self, AgentContact.self, ChatMessage.self])
        let useICloud = UserDefaults.standard.bool(forKey: "iCloudSyncEnabled")
        let config: ModelConfiguration
        if useICloud {
            config = ModelConfiguration(
                schema: schema,
                isStoredInMemoryOnly: false,
                cloudKitDatabase: .automatic
            )
        } else {
            config = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)
        }
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
