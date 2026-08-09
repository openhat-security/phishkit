<!-- Thanks for contributing to phishkit. Contributions are licensed under GPL-3.0. -->

## Summary

<!-- What does this change do, and why? -->

## Privacy / security impact

<!-- Effect on captured data, delivery, the authorized-use gate, elevated
     operations, and failure modes. State "none" only if truly none. -->

## Checklist

- [ ] `make check` / `make lint` pass
- [ ] `make docs-build` passes (when docs changed)
- [ ] Change is in the supported surface (`apps/`, `crates/`, `demos/`, `docs/`) — not resurrecting removed stacks
- [ ] CLI (`phishkit` / `phishkit wiz`) updated when a new desktop capability should be scriptable
- [ ] Authorized-use gate and allow-listed replay preserved (no silent bypass)
- [ ] No captured credentials, tokens, cookies, recipient PII, secrets, or `run/` state committed
- [ ] Attribution preserved and license compatibility verified for reused code

<!-- Report security vulnerabilities privately per SECURITY.md, not in a public PR. -->
