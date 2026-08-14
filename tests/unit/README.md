# Unit tests

Rust tests compiled by Cargo.

- `#[cfg(test)]` modules in `crates/phishkit-core/src/` cover pure helpers
  (host normalization, tag merge, `/etc/hosts` FQDN lists, kit-root checks,
  setup env overrides).
- Files in this directory are public-API tests wired with `[[test]]` paths in
  `crates/phishkit-core/Cargo.toml` and `apps/cli/Cargo.toml`.
- `support.rs` isolates `PHISHKIT_CONFIG` / `PHISHKIT_DATA` under `tempfile`
  and serializes env mutation with a lock.

```bash
make test-unit
# or
cargo test -p phishkit-core -p phishkit-cli --all-targets
```

These tests do not send mail, write `/etc/hosts`, or start evilginx.
