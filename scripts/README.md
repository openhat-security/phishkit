# scripts/

Operator and maintainer automation. **Python and shell are intentional here**
(product apps/engine stay TypeScript + Rust).

| Script | Purpose |
|--------|---------|
| `sync_community_phishlets.py` | Refresh `vendor/community-phishlets/` from lockfile pins |
| `ensure_vendors.sh` | Init/update `vendor/evilginx2` submodule |
| `e2e_destinations.py` / `e2e_demo_videos.py` | Optional e2e / docs video helpers |
| `publish_docs_videos.sh` | Upload walkthrough media to the `docs-media` release |

Campaigns and captures: use `phishkit wiz` / `list-captures` / `delete-capture`
(or `make session-list` / `make session-delete`).

Optional Python deps for Playwright/pexpect e2e:

```bash
python3 -m pip install -r scripts/requirements.txt
```

Authorized lab use only.
