// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "XMacApp",
    platforms: [
        .macOS(.v14)
    ],
    targets: [
        .executableTarget(
            name: "XMacApp",
            path: "Sources/XMacApp",
            resources: [
                .copy("Resources/XMacMemoryGNN.mlpackage"),
            ]
        )
    ]
)
