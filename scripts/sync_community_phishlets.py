#!/usr/bin/env python3
"""Refresh vendored community evilginx phishlet packs on disk.

Packs ship in-repo under vendor/community-phishlets/. Re-run this script (or
`make community-phishlets`) to refresh pins from
kit/evilginx/community-phishlets.lock.json — fetch each repo at the pinned commit
(GitHub tarball) and extract *.yaml / *.yml into:

  vendor/community-phishlets/<source_id>/
  vendor/community-phishlets/index.json   (merged catalog)
  vendor/community-phishlets/_meta.json

Authorized learning/testing only. Third-party YAML is unreviewed for your
engagement; customize and validate before any authorized use.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import shutil
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

KIT_ROOT = Path(__file__).resolve().parents[1]
LOCK_PATH = KIT_ROOT / "kit" / "evilginx" / "community-phishlets.lock.json"
OUT_ROOT = KIT_ROOT / "vendor" / "community-phishlets"
USER_AGENT = "phishkit-community-sync/1.0 (authorized-assessment-kit)"


def _write_root_readme(out_root: Path, sources: list[dict]) -> None:
    """Single credits file at the pack root (survives per-source rmtree)."""
    rows = []
    for src in sources:
        sid = src["id"]
        repo = src["repo"]
        commit = src["commit"]
        url = f"https://github.com/{repo}"
        tree = f"{url}/tree/{commit}"
        rows.append(
            f"| [`{sid}/`]({sid}/) | [{repo}]({url}) | [`{commit[:12]}`]({tree}) |"
        )
    table = "\n".join(rows)
    (out_root / "README.md").write_text(
        f"""# Community phishlet packs

Vendored third-party evilginx phishlet YAML for **authorized learning and
assessment practice only**. These are not first-party phishkit assets.
Credit and copyright belong to the upstream authors.

## Sources

| Directory | Upstream | Pinned commit |
|-----------|----------|---------------|
{table}

Pins are recorded in [`kit/evilginx/community-phishlets.lock.json`](../../kit/evilginx/community-phishlets.lock.json).
Refresh with:

```bash
make community-phishlets
```

Also see [`demos/community/README.md`](../../demos/community/README.md) for an
annotated shortlist. First-party kit phishlets remain under
`kit/evilginx/phishlets/`.
""",
        encoding="utf-8",
    )


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _fetch_tarball(repo: str, commit: str) -> bytes:
    # codeload is the stable tarball host for commit SHAs
    url = f"https://codeload.github.com/{repo}/tar.gz/{commit}"
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as resp:
        return resp.read()


def _extract_yamls(tarball: bytes, dest: Path) -> list[Path]:
    dest.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    with tarfile.open(fileobj=io.BytesIO(tarball), mode="r:gz") as tf:
        for member in tf.getmembers():
            if not member.isfile():
                continue
            name = Path(member.name).name
            if not name.lower().endswith((".yaml", ".yml")):
                continue
            # Skip junk / nested non-phishlet noise if any
            if name.startswith("."):
                continue
            f = tf.extractfile(member)
            if f is None:
                continue
            data = f.read()
            out = dest / name
            # Disambiguate rare same-basename different paths in one archive
            if out.exists() and _sha256(out.read_bytes()) != _sha256(data):
                stem, suf = out.stem, out.suffix
                out = dest / f"{stem}__dup{suf}"
            out.write_bytes(data)
            written.append(out)
    return written


def _load_lock(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def sync(lock_path: Path = LOCK_PATH, out_root: Path = OUT_ROOT, force: bool = False) -> int:
    if not lock_path.is_file():
        print(f"error: lock file missing: {lock_path}", file=sys.stderr)
        return 1

    lock = _load_lock(lock_path)
    sources = sorted(lock.get("sources") or [], key=lambda s: int(s.get("priority", 99)))
    if not sources:
        print("error: no sources in lock file", file=sys.stderr)
        return 1

    out_root.mkdir(parents=True, exist_ok=True)
    meta_sources = []
    per_source_files: dict[str, list[dict]] = {}

    for src in sources:
        sid = src["id"]
        repo = src["repo"]
        commit = src["commit"]
        priority = int(src.get("priority", 99))
        dest = out_root / sid

        print(f"→ {sid}: {repo}@{commit[:12]} …")
        try:
            blob = _fetch_tarball(repo, commit)
        except urllib.error.HTTPError as e:
            print(f"  FAIL HTTP {e.code} for {repo}@{commit}", file=sys.stderr)
            return 1
        except Exception as e:
            print(f"  FAIL {e}", file=sys.stderr)
            return 1

        if dest.exists() and force:
            shutil.rmtree(dest)
        elif dest.exists() and not force:
            # Always refresh contents for pinned sync (idempotent overwrite)
            shutil.rmtree(dest)

        paths = _extract_yamls(blob, dest)
        entries = []
        for p in sorted(paths, key=lambda x: x.name.lower()):
            raw = p.read_bytes()
            entries.append(
                {
                    "name": p.name,
                    "path": str(p.relative_to(out_root)),
                    "sha256": _sha256(raw),
                    "bytes": len(raw),
                }
            )
        per_source_files[sid] = entries
        meta_sources.append(
            {
                "id": sid,
                "repo": repo,
                "commit": commit,
                "priority": priority,
                "tarball_sha256": _sha256(blob),
                "yaml_count": len(entries),
            }
        )
        print(f"  ok — {len(entries)} yaml files → {dest.relative_to(KIT_ROOT)}")

    # Merged index: lower priority number wins on basename collision
    merged: dict[str, dict] = {}
    collisions: list[dict] = []
    for src in sources:
        sid = src["id"]
        priority = int(src.get("priority", 99))
        for ent in per_source_files.get(sid, []):
            key = ent["name"].lower()
            item = {
                "name": ent["name"],
                "source": sid,
                "repo": src["repo"],
                "commit": src["commit"],
                "priority": priority,
                "path": ent["path"],
                "sha256": ent["sha256"],
                "bytes": ent["bytes"],
            }
            if key in merged:
                collisions.append(
                    {
                        "name": ent["name"],
                        "kept": merged[key]["source"],
                        "skipped": sid,
                    }
                )
                continue
            merged[key] = item

    index = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "lock_file": str(lock_path.relative_to(KIT_ROOT)),
        "count": len(merged),
        "collision_count": len(collisions),
        "phishlets": sorted(merged.values(), key=lambda x: x["name"].lower()),
        "collisions": collisions,
    }
    (out_root / "index.json").write_text(json.dumps(index, indent=2) + "\n", encoding="utf-8")

    meta = {
        "generated_at": index["generated_at"],
        "kit_root": str(KIT_ROOT),
        "out_root": str(out_root),
        "sources": meta_sources,
        "merged_count": len(merged),
        "collision_count": len(collisions),
    }
    (out_root / "_meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    _write_root_readme(out_root, sources)

    print()
    print(f"Merged catalog: {len(merged)} unique phishlets ({len(collisions)} collisions skipped)")
    print(f"Index: {out_root.relative_to(KIT_ROOT)}/index.json")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--lock", type=Path, default=LOCK_PATH)
    ap.add_argument("--out", type=Path, default=OUT_ROOT)
    ap.add_argument("--force", action="store_true", help="Remove existing source dirs before extract")
    args = ap.parse_args()
    return sync(lock_path=args.lock, out_root=args.out, force=args.force)


if __name__ == "__main__":
    raise SystemExit(main())
