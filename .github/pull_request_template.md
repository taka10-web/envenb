<!--
Thank you for sending this.

If the change fixes a vulnerability, stop here and use Security Advisories
instead: https://github.com/taka10-web/envenb/security/advisories/new
A public PR is a public disclosure.
-->

## What this changes

<!-- What it does and why. Link the issue if there is one: "Fixes #12". -->

## How it was verified

<!--
Not "tests pass" — what you actually checked, and how someone else could.
If the behaviour cannot be tested automatically, say so and say why.
-->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`
      and `cargo test --no-fail-fast` all pass
- [ ] `pnpm typecheck && pnpm test` pass, if the change touches the frontend
- [ ] Docs updated if behaviour changed (`README.md`, `README.ja.md`, `USAGE.md`,
      the in-app guide, and `docs/` where relevant)
- [ ] No real credential anywhere in the diff, the commits, or this description

## Secret handling

<!-- Delete this section only if the change cannot affect secrets at all. -->

- [ ] No interface returns a secret value — not the CLI, the Tauri commands,
      the MCP tools or `AgentRequest`. Decryption stays inside `envenb-core`,
      behind `with_secret` / `with_credential_field`.
- [ ] Nothing hands a secret to a child process, an application or an AI agent.
      Applications reach providers through the proxy.
- [ ] Anything that puts plaintext in front of a person goes through the human
      gate in `crates/cli/src/human.rs`.
- [ ] A migration under `crates/core/migrations/` that has already shipped was
      not edited — not even a comment (see the checksum test in `db.rs`).
