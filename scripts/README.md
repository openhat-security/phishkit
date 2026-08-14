# scripts/

Operator and maintainer automation. **Python and shell are intentional here**
(product apps/engine stay TypeScript + Rust).

| Script | Purpose |
|--------|---------|
| `sync_community_phishlets.py` | Refresh `vendor/community-phishlets/` from lockfile pins |
| `ensure_vendors.sh` | Init/update `vendor/evilginx2` submodule |
| `destinations_test.py` / `demo_videos.py` | Optional Destinations / local video helpers |
| `publish_docs_videos.sh` | Disabled — do not upload MP4s to GitHub |

Campaigns and captures: use `phishkit wiz` / `list-captures` / `delete-capture`
(or `make session-list` / `make session-delete`).

Optional Python deps for Playwright/pexpect helpers:

```bash
python3 -m pip install -r scripts/requirements.txt
```

Authorized lab use only.
