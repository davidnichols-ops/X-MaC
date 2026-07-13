class Xmac < Formula
  desc "Open-source macOS cleaner, optimizer & system monitor with on-device GNN"
  homepage "https://github.com/davidnichols-ops/X-MaC"
  url "https://github.com/davidnichols-ops/X-MaC/archive/refs/tags/v2.1.1.tar.gz"
  sha256 "0fb41f378a15d7fb726cd0666872db88c81f5633fa481a91ab7dac5f58f33ce3"
  license "MIT"
  head "https://github.com/davidnichols-ops/X-MaC.git", branch: "main"

  depends_on "rust" => :build
  depends_on macos: :monterey

  def install
    system "cargo", "build", "--release", *std_cargo_args
    bin.install "target/release/x-mac" => "xmac"

    generate_completions_from_executable(bin/"xmac", "completions", "--shell", :bash)
    generate_completions_from_executable(bin/"xmac", "completions", "--shell", :zsh)
    generate_completions_from_executable(bin/"xmac", "completions", "--shell", :fish)
  end

  test do
    assert_match "xmac", shell_output("#{bin}/xmac --version")
    assert_match "Engine", shell_output("#{bin}/xmac clean --no-disk 2>&1")
  end
end
