.PHONY: help \
	build-evilginx build \
	validate-phishlet evilginx-dryrun evilginx-prod evilginx-creds \
	evilginx-start evilginx-stop \
	demo demo-cookie demo-firebase demo-tunnel \
	vendor community-phishlets desktop cli \
	setup start check lint docs docs-build \
	campaign quick-test e2e e2e-destinations \
	update-video-documentation update-video-documentation-desktop \
	update-video-documentation-demos publish-docs-videos \
	e2e-tauri docs-videos docs-videos-demos \
	clean session-delete session-list

# =============================================================================
# phishkit — top level convenience targets
# =============================================================================

help:
	@echo "phishkit — Authorized AiTM + Awareness Assessment Kit"
	@echo ""
	@echo "=== Supported product: desktop app + CLI ==="
	@echo "  make desktop                    # Tauri + Rust: Assessments → Targets →"
	@echo "                                  # Phishlet/Proxy → Lures → Templates → Recipients →"
	@echo "                                  # Campaigns → Results → Sessions (native mail)"
	@echo "  make cli                        # Build release phishkit / phishkit_ctl"
	@echo ""
	@echo "Local demos (TypeScript practice apps):"
	@echo "  make demo-cookie             Cookie-session mock on :9080"
	@echo "  make demo-firebase           Firebase-shaped mock on :9081"
	@echo "  make demo                    Start both demos"
	@echo "  # see demos/ and demos/README.md"
	@echo ""
	@echo "Evilginx targets:"
	@echo "  make build-evilginx          Build evilginx2 binary"
	@echo "  make validate-phishlet       PHISHLET=xxx  (default: demo-cookie)"
	@echo "  make evilginx-dryrun         Local self-signed test (interactive)"
	@echo "  make evilginx-start          Start evilginx dry-run in background (screen)"
	@echo "  make evilginx-stop           Stop background evilginx"
	@echo "  make evilginx-creds          Dump captured sessions"
	@echo ""
	@echo "Captures (via phishkit CLI):"
	@echo "  make session-list PROFILE=id # List captures for a profile"
	@echo "  make session-delete PROFILE=id ID=3  # Delete capture (won't re-import on refresh)"
	@echo ""
	@echo "Community phishlets (authorized learning/testing):"
	@echo "  make community-phishlets     Refresh pinned packs in vendor/community-phishlets/"
	@echo "  # packs ship in-repo; sync is optional refresh — see demos/community/"
	@echo ""
	@echo "Developer workflow:"
	@echo "  make setup                   # rust check + desktop deps + docs deps"
	@echo "  make start                   # run the supported desktop app (alias of desktop)"
	@echo "  make check                   # cargo fmt --check + cargo test (core + cli)"
	@echo "  make lint                    # cargo clippy (workspace packages)"
	@echo "  make docs                    # VitePress docs preview (hot reload)"
	@echo "  make docs-build              # build docs; fails on unresolved internal links"
	@echo ""
	@echo "Docs walkthrough videos:"
	@echo "  make update-video-documentation          # desktop tour + demo logins"
	@echo "  make update-video-documentation-desktop  # WebdriverIO Tauri console tour"
	@echo "  make update-video-documentation-demos    # Playwright demo-cookie / demo-firebase"
	@echo "  make publish-docs-videos                 # upload walkthrough-*.mp4 → Release docs-media"
	@echo "  E2E_EMAIL=… E2E_PASSWORD=… make e2e-destinations   # lure + evilginx (mailbox)"
	@echo "  # optional: E2E_TARGET=demo-cookie.local.phishkit E2E_HEADED=1 E2E_KEEP_PROXY=1"
	@echo ""
	@echo "All-in-one build:"
	@echo "  make build"
	@echo "  make clean"

# -----------------------------------------------------------------------------
# Local demos (TypeScript — Node via demos/package.json)
# -----------------------------------------------------------------------------
demo-cookie:
	@(cd demos && npm install --silent && npm run demo:cookie)

demo-firebase:
	@(cd demos && npm install --silent && npm run demo:firebase)

# Start both demos in the background; Ctrl-C stops the waiter (children may linger).
demo:
	@(cd demos && npm install --silent); \
	(cd demos && npm run demo:cookie) & \
	(cd demos && npm run demo:firebase) & \
	echo "demo-cookie  → http://127.0.0.1:9080"; \
	echo "demo-firebase → http://127.0.0.1:9081"; \
	echo "creds: demo@phishkit.local / demo-password"; \
	wait

# Public URL for a local demo (Cloudflare quick tunnel via untun — no signup).
# Usage: make demo-tunnel PORT=9080
demo-tunnel:
	@test -n "$(PORT)" || (echo "PORT= required (e.g. PORT=9080)" && exit 1)
	@(cd demos && npm install --silent && npx --yes untun tunnel http://127.0.0.1:$(PORT))

# -----------------------------------------------------------------------------
# Evilginx
# -----------------------------------------------------------------------------
build-evilginx:
	./kit/evilginx/scripts/build.sh

validate-phishlet:
	PHISHLET="$${PHISHLET:-demo-cookie}" ./kit/evilginx/scripts/validate.sh

evilginx-dryrun:
	./kit/evilginx/scripts/start_dryrun.sh

evilginx-prod:
	./kit/evilginx/scripts/start_prod.sh

evilginx-creds:
	./kit/evilginx/scripts/view_creds.sh

evilginx-start:
	DRYRUN_DOMAIN="$${DRYRUN_DOMAIN:-demo-cookie.local.phishkit}" \
	PHISHLET_NAME="$${PHISHLET_NAME:-demo-cookie}" \
	./kit/evilginx/scripts/start_evilginx_dryrun.sh

evilginx-stop:
	./kit/evilginx/scripts/stop_evilginx.sh

# -----------------------------------------------------------------------------
# Vendor submodules (evilginx2 under vendor/)
# -----------------------------------------------------------------------------
vendor:
	@./scripts/ensure_vendors.sh

# Pull latest upstream commits on submodule tracking branches (optional).
vendor-update:
	@git submodule update --remote --merge
	@./scripts/ensure_vendors.sh

# Combined build: ensure upstream sources exist, then build the evilginx binary.
build: vendor build-evilginx

# -----------------------------------------------------------------------------
# Community phishlet packs (pinned commits; vendored under vendor/community-phishlets/)
# -----------------------------------------------------------------------------
community-phishlets:
	@python3 scripts/sync_community_phishlets.py --force

# -----------------------------------------------------------------------------
# Desktop app (Tauri + Rust — no Python API)
# -----------------------------------------------------------------------------
# rustup installs cargo to ~/.cargo/bin — Make/npm often lack that PATH.
cli:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	cargo build --release -p phishkit-cli
	@echo "binaries: target/release/phishkit and target/release/phishkit_ctl (workspace root)"

desktop:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	command -v cargo >/dev/null || { echo "cargo not found. Install Rust: https://rustup.rs (then restart the shell)"; exit 1; }; \
	cd apps/desktop && npm install && npm run tauri dev

# -----------------------------------------------------------------------------
# Developer workflow (setup / start / check / lint / docs)
# -----------------------------------------------------------------------------
# One-time (and safe to re-run) environment setup: verify the Rust toolchain,
# install desktop app dependencies, and install the docs site dependencies.
setup:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	command -v cargo >/dev/null || { echo "cargo not found. Install Rust: https://rustup.rs (then restart the shell)"; exit 1; }; \
	echo "==> desktop dependencies"; (cd apps/desktop && npm install); \
	echo "==> docs dependencies"; npm install

# Run the supported desktop app.
start: desktop

# Fast quality gates for the native engine (mirrors CI).
check:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }; \
	cargo fmt --all -- --check && \
	cargo test -p phishkit-core -p phishkit-cli --all-targets

lint:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }; \
	cargo clippy -p phishkit-core -p phishkit-cli -p phishkit --all-targets -- -D warnings

# Hot-reloading docs preview (http://localhost:5173).
docs:
	@npm install >/dev/null 2>&1 || true; \
	npm run docs:dev

# What the docs workflow builds; fails on unresolved internal links.
docs-build:
	@npm install >/dev/null 2>&1 || true; \
	npm run docs:build

# -----------------------------------------------------------------------------
# Campaign bootstrap (points at native CLI wizards)
# -----------------------------------------------------------------------------
campaign:
	@echo "Use the native CLI or desktop app:"
	@echo "  make cli && ./target/release/phishkit wiz quickstart"
	@echo "  ./target/release/phishkit wiz send"
	@echo "  make desktop"
	@exit 1

# -----------------------------------------------------------------------------
# The real "easy button" — fully automated local end-to-end test
# -----------------------------------------------------------------------------
# Example:
#   make quick-test TARGET=app.myapp.com EMAIL=alice@mycompany.com
quick-test:
	@TARGET_CLEAN="$$(echo '$(TARGET)' | sed -E 's#^https?://##; s#/.*##; s#:.*##')"; \
	python3 scripts/quick_assessment.py --target-domain "$$TARGET_CLEAN" --email "$(EMAIL)" $(if $(KEEP_RUNNING),--keep-running,)

# Destinations page e2e: steps 1–4 via phishkit_ctl + Playwright lure login.
# Requires authorized test mailbox credentials in the environment.
# Example:
#   E2E_EMAIL=user@client.com E2E_PASSWORD=secret make e2e-destinations
#   E2E_EMAIL=… E2E_PASSWORD=… E2E_HEADED=1 E2E_KEEP_PROXY=1 make e2e-destinations
e2e-destinations:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }; \
	test -n "$$E2E_EMAIL" || { echo "E2E_EMAIL required"; exit 1; }; \
	test -n "$$E2E_PASSWORD" || { echo "E2E_PASSWORD required"; exit 1; }; \
	if [ ! -x venv/bin/python ]; then python3 -m venv venv; fi; \
	./venv/bin/python -m pip install -q -r scripts/requirements.txt; \
	./venv/bin/python -m playwright install chromium; \
	cargo build -p phishkit-cli --bin phishkit_ctl; \
	E2E_TARGET="$${E2E_TARGET:-demo-cookie.local.phishkit}" \
	  ./venv/bin/python scripts/e2e_destinations.py

# -----------------------------------------------------------------------------
# Docs walkthrough videos (gitignored under docs/media/)
# -----------------------------------------------------------------------------
# Prefer: make e2e   (or make update-video-documentation)
# Pieces: update-video-documentation-desktop | update-video-documentation-demos
# Publish: make publish-docs-videos  → GitHub Release tag docs-media

# Desktop console tour (WebdriverIO + embedded WebDriver). Starts demo-cookie,
# builds a release e2e binary with embedded frontend, writes walkthrough-assessment.mp4.
update-video-documentation-desktop:
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }; \
	mkdir -p docs/media run; \
	(cd demos && npm install --silent && npm run demo:cookie) & echo $$! > run/demo-cookie.pid; \
	sleep 1; \
	trap 'kill $$(cat run/demo-cookie.pid) 2>/dev/null || true; rm -f run/demo-cookie.pid' EXIT; \
	(cd apps/desktop && npm install && VITE_E2E=1 npm run build); \
	# custom-protocol embeds frontendDist (plain cargo build without it stays on devUrl / about:blank).
	cargo build --release -p phishkit --features e2e,custom-protocol; \
	(cd docs/capture && npm install && \
	  PHISHKIT_E2E_BIN="$(CURDIR)/target/release/phishkit" npm test) && \
	f=$$(ls -t docs/media/full-product-tour*.mp4 docs/media/full-product-tour*.webm \
	       docs/media/*Sessions*.webm docs/media/*Sessions*.mp4 \
	       docs/media/*.webm docs/media/*.mp4 2>/dev/null | head -1); \
	if [ -n "$$f" ]; then \
	  out=docs/media/walkthrough-assessment.mp4; \
	  if command -v ffmpeg >/dev/null 2>&1; then \
	    # Remux/reencode so moov is complete (wdio-video-reporter mp4 can be unseekable). \
	    ffmpeg -y -i "$$f" -c:v libx264 -pix_fmt yuv420p -movflags +faststart "$$out" >/dev/null 2>&1; \
	  else \
	    case "$$f" in \
	      *.mp4) cp -f "$$f" "$$out" ;; \
	      *.webm) cp -f "$$f" docs/media/walkthrough-assessment.webm ;; \
	    esac; \
	  fi; \
	fi

# Playwright login videos against localhost demos (no Tauri binary required).
update-video-documentation-demos:
	@mkdir -p docs/media run; \
	(cd demos && npm install --silent); \
	(cd demos && npm run demo:firebase) & echo $$! > run/demo-firebase.pid; \
	(cd demos && npm run demo:cookie) & echo $$! > run/demo-cookie2.pid; \
	sleep 1; \
	trap 'kill $$(cat run/demo-firebase.pid run/demo-cookie2.pid) 2>/dev/null || true; \
	  rm -f run/demo-firebase.pid run/demo-cookie2.pid' EXIT; \
	if [ ! -x venv/bin/python ]; then python3 -m venv venv; fi; \
	./venv/bin/python -m pip install -q -r scripts/requirements.txt; \
	./venv/bin/python -m playwright install chromium; \
	./venv/bin/python scripts/e2e_demo_videos.py

# Full docs media suite: desktop tour + demo logins.
update-video-documentation: update-video-documentation-desktop update-video-documentation-demos

# Canonical short aliases for the docs video suite.
e2e: update-video-documentation
e2e-video-documentation: update-video-documentation
e2e-tauri: update-video-documentation-desktop

publish-docs-videos:
	@chmod +x scripts/publish_docs_videos.sh
	@./scripts/publish_docs_videos.sh

# -----------------------------------------------------------------------------
# Captures (phishkit CLI)
# -----------------------------------------------------------------------------
PHISHKIT_BIN := ./target/release/phishkit

session-list:
	@test -n "$(PROFILE)" || (echo "PROFILE= required (profile id slug)" && exit 1)
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	if [ ! -x "$(PHISHKIT_BIN)" ]; then cargo build -p phishkit-cli --bin phishkit --release; fi; \
	$(PHISHKIT_BIN) list-captures -p "$(PROFILE)"

session-delete:
	@test -n "$(PROFILE)" || (echo "PROFILE= required" && exit 1)
	@test -n "$(ID)" || (echo "ID= required (evilginx session #)" && exit 1)
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	if [ ! -x "$(PHISHKIT_BIN)" ]; then cargo build -p phishkit-cli --bin phishkit --release; fi; \
	$(PHISHKIT_BIN) delete-capture -p "$(PROFILE)" -s "$(ID)"

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------
clean:
	rm -rf run/ kit/evilginx/run/ kit/evilginx/bin/ docs/media/ docs/capture/node_modules/ docs/.vitepress/dist docs/.vitepress/cache
