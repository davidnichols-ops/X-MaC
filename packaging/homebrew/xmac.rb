# Homebrew formula for xmac CLI
#
# To use this formula with a custom tap:
#   brew tap davidnichols-ops/xmac
#   brew install xmac
#
# Or install directly:
#   brew install --HEAD https://raw.githubusercontent.com/davidnichols-ops/X-MaC/main/packaging/homebrew/xmac.rb
#
# Note: This formula builds from source. A pre-built bottle will be added
# once the first release is tagged.

class Xmac < Formula
  desc "Open-source macOS cleaner, optimizer & system monitor with on-device GNN"
  homepage "https://github.com/davidnichols-ops/X-MaC"
  url "https://github.com/davidnichols-ops/X-MaC/archive/refs/tags/v2.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/davidnichols-ops/X-MaC.git", branch: "main"

  # X-MaC requires macOS 13+ (Ventura) for the GUI, but the CLI works on
  # older versions too. We set 12.0 as the minimum for Homebrew to avoid
  # blocking users who only want the CLI.
  depends_on macos: "12.0"

  # Build dependencies
  depends_on "rust" => :build

  # Runtime dependencies (none — the binary is self-contained)
  # The GNN model is bundled, no Python or external libs needed at runtime.

  def install
    # Build the release binary
    system "cargo", "build", "--release", *std_cargo_args

    # Install the binary
    bin.install "target/release/x-mac" => "xmac"

    # Install shell completions
    output = Utils.safe_popen_read("#{bin}/xmac", "completions", "--shell", "bash")
    (bash_completion/"xmac").write output

    output = Utils.safe_popen_read("#{bin}/xmac", "completions", "--shell", "zsh")
    (zsh_completion/"_xmac").write output

    output = Utils.safe_popen_read("#{bin}/xmac", "completions", "--shell", "fish")
    (fish_completion/"xmac.fish").write output
  end

  test do
    # Verify the binary runs and reports version
    assert_match "xmac #{version}", shell_output("#{bin}/xmac --version")

    # Verify a basic scan command works (read-only, no side effects)
    assert_match "Engine", shell_output("#{bin}/xmac clean --no-disk 2>&1", 0)
  end
end
