# Formula for the X-MaC CLI (built from source).
# The GUI is distributed separately as a cask (see xmac.rb).
class XmacCli < Formula
  desc "Open-source macOS cleaner, optimizer & system monitor with on-device GNN intelligence"
  homepage "https://github.com/davidnichols-ops/X-MaC"
  url "https://github.com/davidnichols-ops/X-MaC.git",
      tag:      "v2.1.1",
      revision: "bc9542bee6276380842209579958bbadc8303bd0"
  version "2.1.1"
  license "MIT"

  # X-MaC requires macOS 13 (Ventura) or newer.
  depends_on macos: :ventura

  # Rust toolchain for building from source.
  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--bin", "x-mac"
    bin.install "target/release/x-mac" => "xmac"
  end

  test do
    assert_match "X-MaC", shell_output("#{bin}/xmac --version")
  end
end
