# Packaging (alpha)

Stubs for distribution channels. The [Release workflow](../.github/workflows/release.yml)
builds CLI archives on `v*` tags and attaches refreshed Homebrew / AUR / Debian
files to the GitHub Release.

## Channels

| Channel | Path | Alpha behavior |
|---------|------|--------------------|
| GitHub Releases | workflow artifacts | Primary — CLI `.tar.gz` + sha256 |
| Homebrew | [`homebrew/phishkit.rb`](homebrew/phishkit.rb) | Formula attached; fill `sha256` after tag archive exists; push to your tap when ready |
| AUR | [`aur/PKGBUILD`](aur/PKGBUILD) | PKGBUILD attached with asset URL/sha; publish manually or with `AUR_SSH_PRIVATE_KEY` later |
| Debian / Launchpad | [`debian/`](debian/) | Changelog bumped; `dput` remains operator-driven (`REPLACE_ME` identities) |

## Secrets (optional publish)

| Secret | Use |
|--------|-----|
| `HOMEBREW_TAP_TOKEN` | Push formula to a Homebrew tap (not wired by default) |
| `AUR_SSH_PRIVATE_KEY` | Push PKGBUILD to AUR (not wired by default) |

Until those exist, the Release job still succeeds with downloadable packaging files.

## Authorized use

Packages must not be marketed as unauthorized phishing tools. Point install docs
at [authorized use](../docs/guide/authorized-use.md).
