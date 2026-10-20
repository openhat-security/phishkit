# frozen_string_literal: true

# phishkit-cli: headless CLI only, built from the tagged source.
# Publish to the tap as `openhat-security/tap/phishkit-cli`.
# Update `url`/`sha256`/`version` for each tag (the release workflow prints both).
class PhishkitCli < Formula
  desc "Authorized AiTM + awareness assessment CLI (alpha, CLI only)"
  homepage "https://github.com/openhat-security/phishkit"
  url "https://github.com/openhat-security/phishkit/archive/refs/tags/v0.1.1-alpha.1.tar.gz"
  sha256 "02d932e97cdba099c9f2640f83041241b793e8d0f298be748224b9888c1418a9"
  version "0.1.1-alpha.1"
  license "GPL-3.0-only"

  depends_on "rust" => :build
  depends_on "go" => :build

  def install
    system "cargo", "install", "--locked", "--path", "apps/cli", "--root", prefix,
           "--bin", "phishkit", "--bin", "phishkit_ctl"
    (share/"phishkit").install Dir["kit/evilginx/phishlets"]
  end

  def caveats
    <<~EOS
      phishkit-cli is the headless CLI only. For the desktop app, install the cask:
        brew install --cask openhat-security/tap/phishkit

      Alpha build for authorized security assessments only. Read the authorized-use
      policy before running it: #{opt_share}/phishkit or the repo docs.
    EOS
  end

  test do
    assert_match "phishkit", shell_output("#{bin}/phishkit --help 2>&1")
    assert_match "phishkit_ctl", shell_output("#{bin}/phishkit_ctl --help 2>&1")
  end
end
