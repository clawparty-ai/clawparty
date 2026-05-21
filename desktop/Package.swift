// swift-tools-version:5.8
import PackageDescription

let package = Package(
    name: "ClawPartyDesktop",
    platforms: [
        .macOS(.v13)
    ],
    dependencies: [
        .package(url: "https://github.com/migueldeicaza/SwiftTerm.git", from: "1.0.0")
    ],
    targets: [
        .executableTarget(
            name: "ClawPartyDesktop",
            dependencies: [
                "SwiftTerm"
            ],
            path: "ClawPartyDesktop",
            exclude: ["Info.plist", "ClawPartyDesktop.icns"],
            swiftSettings: [
                .enableExperimentalFeature("BareSlashRegexLiterals")
            ]
        )
    ]
)
