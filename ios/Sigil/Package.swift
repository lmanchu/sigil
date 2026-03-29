// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Sigil",
    platforms: [.iOS(.v18), .macOS(.v15)],
    dependencies: [
        .package(url: "https://github.com/rust-nostr/nostr-sdk-swift.git", from: "0.44.0"),
    ],
    targets: [
        .executableTarget(
            name: "Sigil",
            dependencies: [
                .product(name: "NostrSDK", package: "nostr-sdk-swift"),
            ],
            path: "Sources",
            resources: [
                .process("../Resources/Assets.xcassets"),
            ]
        ),
    ]
)
