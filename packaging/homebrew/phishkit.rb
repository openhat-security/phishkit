# frozen_string_literal: true

# phishkit: the full Tauri desktop app.
# Publish to the tap as `openhat-security/tap/phishkit` (cask).
#
# PENDING: the desktop release artifacts (.dmg for macOS, .AppImage for Linux) are
# not built or signed yet. The current release workflow (.github/workflows/release.yml)
# only builds the CLI. A desktop build+sign job is still needed (the signed-updater
# build job tracked in the follow-ups) before this cask can resolve.
#
# Once the desktop artifacts are attached to the latest tag, replace each
# REPLACE_ME_DESKTOP_* sha256 with the real checksum and bump `version`.
# Expected Tauri bundle names for v#{version}:
#   macOS arm64:  phishkit_#{version}_aarch64.dmg
#   macOS x86_64: phishkit_#{version}_x64.dmg
#   Linux x86_64: phishkit_#{version}_amd64.AppImage
cask "phishkit" do
  version "0.1.1-alpha.1"

  on_macos do
    on_arm do
      sha256 "REPLACE_ME_DESKTOP_DMG_ARM64_NOT_BUILT_YET"
      url "https://github.com/openhat-security/phishkit/releases/download/v#{version}/phishkit_#{version}_aarch64.dmg"
    end
    on_intel do
      sha256 "REPLACE_ME_DESKTOP_DMG_X64_NOT_BUILT_YET"
      url "https://github.com/openhat-security/phishkit/releases/download/v#{version}/phishkit_#{version}_x64.dmg"
    end
  end
  on_linux do
    on_intel do
      sha256 "REPLACE_ME_DESKTOP_APPIMAGE_X64_NOT_BUILT_YET"
      url "https://github.com/openhat-security/phishkit/releases/download/v#{version}/phishkit_#{version}_amd64.AppImage"
    end
  end

  name "phishkit"
  desc "Authorized AiTM + awareness assessment kit (full desktop app, alpha)"
  homepage "https://github.com/openhat-security/phishkit"

  livecheck do
    skip "Auto-generated on release."
  end

  depends_on macos: :big_sur

  app "phishkit.app"

  postflight do
    # Alpha builds are not notarized yet, so clear the quarantine bit to let
    # Gatekeeper open the app on first launch.
    if OS.mac?
      system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{staged_path}"]
    end
  end

  caveats <<~EOS
    phishkit is an alpha build for authorized security assessments only.
    Read the authorized-use policy before running it.

    For the headless CLI only, install the formula instead:
      brew install openhat-security/tap/phishkit-cli
  EOS

  zap trash: [
    "~/Library/Application Support/com.phishkit.app",
    "~/Library/Preferences/com.phishkit.app.plist",
    "~/Library/Saved Application State/com.phishkit.app.savedState",
  ]
end
