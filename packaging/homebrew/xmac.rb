cask "xmac" do
  version "2.1.1"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  # TODO: generate with `shasum -a 256 X-MaC-2.1.1.dmg` and replace the placeholder above.

  # TODO: replace with the published GitHub release asset URL after the first
  # release is uploaded. Keep the URL template in sync with `version`.
  url "https://github.com/davidnichols-ops/X-MaC/releases/download/v#{version}/X-MaC-#{version}.dmg"
  name "X-MaC"
  desc "Open-source macOS cleaner, optimizer & system monitor with on-device GNN intelligence"
  homepage "https://github.com/davidnichols-ops/X-MaC"

  # X-MaC requires macOS 13 (Ventura) or newer for SwiftUI + CoreML features.
  depends_on macos: ">= :ventura"

  app "X-MaC.app"

  # No telemetry, no background daemons installed by the cask itself.
  # The app's optional launch agent is opt-in and managed in-app.
  zap trash: [
    "~/Library/Application Support/X-MaC",
    "~/Library/Preferences/com.xmac.gui.plist",
    "~/Library/Caches/com.xmac.gui",
    "~/Library/Logs/X-MaC",
    "~/Library/Saved Application State/com.xmac.gui.savedState",
    "~/Library/HTTPStorages/com.xmac.gui",
    "~/Library/WebKit/com.xmac.gui",
  ]
end
