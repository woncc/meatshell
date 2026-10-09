# Session editor and SSH connection regressions

These tests use generated credentials and loopback servers only. Do not substitute
real profiles or keys. They do not prove compatibility with every remote server.

## Rust and lightweight UI checks

```sh
cargo test --locked
cargo check --locked --bin meatshell
```

`cargo test` includes the production SSH/config unit tests, the editor-test
ownership model, and lightweight Slint fixtures. `ui_auth_prompts` compiles the
production prompt queues against a minimal window and inert persistence, avoiding
the memory cost of the full application. `ui_session_editor` renders the real
session dialog and verifies Save while a test is pending, failed, timed out or
successful, plus close/cancel and connection-field change notifications.

Desktop end-to-end/manual testing is still useful for platform-specific focus and fonts.

## Loopback SSH/SFTP checks

Install the test dependency `paramiko` (and its `cryptography` dependency), then:

The Python drivers `tests/ssh_jump_chain_e2e.py` and `tests/config_import_e2e.py`
ran through `meatshell cli` / `meatshell mcp serve`. Those entry points were
removed, and the scripts with them. Jump-chain rules stay covered by the Rust
tests in `src/config/jump_chain.rs`. Import of legacy files, including removed
MCP keys, stays covered by `src/config/impls/import_tests.rs`.

Network-stage deadlines do not limit human credential/MFA entry or host-key
confirmation. The handshake budget pauses for host-key decisions, and cancelling
a test closes its own pending transport and prompts. Disconnect cleanup is
best-effort and cannot delay the completed test indefinitely.

## Editor invariants

- Only a newly added, never-selected jump row may be omitted from Save/Test
- Saved empty/dangling hops, broken legacy routes and cycles remain errors
- Save validates configuration and never requires a successful connection test
- Save, Cancel, reopening, another Test, route edits and connection-field edits
  invalidate earlier results and queued test prompts
- Cancelling a test cannot cancel or clear another terminal/window's login input
- A test-owned authentication overlay offers Save/Create without accepting a
  host key, supplying credentials or completing MFA
