# Security design

- **Secrets never reach SQLite in the clear.** They are stored in a separate
  `secrets` table that has no plaintext column, as XChaCha20-Poly1305
  ciphertext + nonce (RustCrypto `chacha20poly1305`; no home-grown crypto).
  Tests assert the plaintext is absent from every file in the data directory,
  including the WAL.
- **Separate, non-serializable secret types.** `SecretValue` (core) and
  `MasterKey` (vault) print `[REDACTED]` under `Debug`, have no `Display`,
  `Serialize`, `Clone` or `PartialEq`, and are zeroized on drop. Reading the
  content requires calling `expose()`, which is easy to grep for in review.
- **Secrets never enter an application process.** `envenb run` passes PUBLIC
  variables only. A value in `process.env` can be read back by any code in that
  process, so handing one over ends the guarantee regardless of who started the
  command. Applications call providers through the local proxy, which holds the
  credential on their behalf.
- **No AI-reachable "get secret".** Decryption is `pub(crate)`; the sanctioned
  consumers are the closure-based `with_secret` used by the Broker and the
  proxy, `with_credential_field` behind the human-only copy / ssh paths, and
  `resolve_process_env` for `export-env` (which writes a real `.env` for tools
  that cannot be proxied, and is human-only). Neither
  the Tauri commands, the MCP tools nor `AgentRequest` can return a value. The
  desktop app cannot reveal a stored secret; a human-only reveal with OS
  authentication is still deliberately absent.
- **Permission decisions never read AI text.** The engine consults stored rules
  and environment names only. Defaults: development-like READ ALLOW / WRITE
  ASK / DELETE DENY; production-like READ ASK / WRITE DENY / DELETE DENY.
  `ASK` requires a human click or `envenb ai approve`, with a 3-minute timeout.
- **Broker hygiene.** Credentials are marked sensitive in headers, scrubbed
  from response bodies and error messages, requests cannot leave the
  connection host, redirects are not followed, bodies are capped at 256 KiB.
- **Plaintext crosses the webview boundary exactly once**, in
  `set_secret_variable`, where it is wrapped in `SecretValue` immediately and
  the incoming `String` is zeroized. The React side uses a password input,
  never caches the value, and never receives it back.
- **CLI never takes secrets from argv.** `var set-secret` reads stdin, with
  echo disabled on a TTY, so values do not land in shell history or `ps`.
- **Logs and errors carry identifiers only.** `CoreError` / `VaultError`
  variants embed names and paths, never values; AEAD failures are reported as
  one opaque `Decrypt` error.
- **Plaintext-emitting commands are human-only.** `export-env`, `ssh`,
  `cred copy` and `var copy` require an interactive terminal and refuse to run
  inside known agent sessions (`CLAUDECODE`, Codex, Cursor, Gemini CLI
  markers). `ENVENB_ALLOW_UNATTENDED=1` opts a script you run yourself back in.
  `envenb run` is *not* gated: it emits no plaintext, so an agent starting a
  dev server is not a leak. This detection is a convenience, not the boundary —
  the boundary is that the secret is never in the process to begin with.
- **A session token is a capability, not a credential.** The application holds
  one and can therefore call the proxy — that is the point of having a proxy at
  all. It is scoped to a project / environment / connection set, expires, works
  only against a running local daemon, and every call it makes is in the audit
  log. Losing one is bounded and visible; losing a provider key is neither. See
  [../SECURITY.md](../SECURITY.md) for the comparison.
- **The proxy is not an open relay.** The upstream host comes from the
  connection's `base_url`; a caller chooses a path, never a host. Redirects are
  refused, and any header or query parameter that would authenticate the caller
  to the provider — including the header *this* connection authenticates with —
  is discarded before the request leaves. Session tokens authenticate callers
  to EnvEnb only, are scoped to a project / environment / connection set, expire,
  and are stored as hashes.
- **Master key is separate from the database.** Copying `envenb.db` alone
  yields nothing. New vaults default to the OS keychain
  (`ENVENB_KEY_BACKEND=file` overrides, and existing `master.key` files are kept);
  `envenb vault key-backend keychain` moves a file-backed key into the macOS
  Keychain (read back before the file is deleted). Secrets need no re-encryption
  because the key bytes are unchanged.
- **Desktop capabilities are minimal.** Only `core:default`; no shell, fs or
  clipboard plugins. CSP is set.

---

To report a vulnerability, see [../SECURITY.md](../SECURITY.md).
