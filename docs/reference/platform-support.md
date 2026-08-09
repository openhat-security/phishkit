# Platform support

phishkit's operator workflow is developed and tested primarily on **macOS**.
Other platforms build and run to varying degrees; this page states the current
matrix. phishkit is pre-1.0 alpha software.

## Matrix

| Capability | macOS | Linux | Windows |
|------------|:-----:|:-----:|:-------:|
| Build (`make build`, desktop app) | ✅ | ✅ | ⚠️ untested |
| Desktop app (`make desktop`) | ✅ | ✅ | ⚠️ untested |
| `phishkit_ctl` CLI | ✅ | ✅ | ⚠️ untested |
| evilginx AiTM proxy (local dry-run) | ✅ | ✅ | ❌ |
| `/etc/hosts` add/remove with native admin prompt | ✅ (osascript) | ⚠️ manual/sudo | ❌ |
| Email delivery (SMTP/ESP) | ✅ | ✅ | ⚠️ untested |
| Session capture / attribution / replay | ✅ | ✅ | ❌ |

Legend: ✅ supported · ⚠️ partial or untested · ❌ not supported in this phase.

## Notes

- **macOS** is the reference platform. Elevated `/etc/hosts` changes use a
  native `osascript` administrator prompt, with paired cleanup.
- **Linux** builds and runs the desktop app, CLI, and delivery engine. Elevated
  host changes do not use the macOS prompt path; apply them with your platform's
  standard privilege mechanism.
- **Windows** is not a supported operator target in this phase. The AiTM
  dry-run and host management assume a Unix-like environment.

## Prerequisites

See [install](/guide/install) for the full toolchain: git (submodules), Rust
stable, Node (see `.nvmrc`), the Go toolchain (to build evilginx once), and the
Tauri prerequisites for your OS.
