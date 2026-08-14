# frozen_string_literal: true

# Placeholder Homebrew formula — publish to your tap after filling URLs/SHA.
class Phishkit < Formula
  desc "Authorized AiTM + awareness assessment CLI"
  homepage "https://github.com/openhat-security/phishkit"
  url "https://github.com/openhat-security/phishkit/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_ME"
  license "GPL-3.0-only"

  depends_on "rust" => :build
  depends_on "go" => :build

  def install
    system "cargo", "install", "--locked", "--path", "apps/cli", "--root", prefix, "--bin", "phishkit"
    (share/"phishkit").install Dir["kit/evilginx/phishlets"]
  end

  test do
    assert_match "phishkit", shell_output("#{bin}/phishkit --help 2>&1")
  end
end
