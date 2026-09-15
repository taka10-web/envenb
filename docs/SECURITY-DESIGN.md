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
- **No AI-reachable "get secret".** Decryption is `pub(crate)`; the sanctioned
  consumers are `resolve_process_env` (for `envenb run`, human initiated), the
  closure-based `with_secret` used by the Broker, and `with_credential_field`
  behind the human-only copy / ssh / run paths. Neither
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
- **Plaintext-emitting commands are human-only.** `envenb run`, `export-env`,
  `ssh`, `cred copy` and `var copy` require an interactive terminal and refuse to run inside
  known agent sessions (`CLAUDECODE`, Codex, Cursor, Gemini CLI markers). An AI
  with shell access therefore cannot call `envenb run env` to dump the vault;
  it gets the MCP broker instead. `ENVENB_ALLOW_UNATTENDED=1` opts a script
  you run yourself back in. Metadata commands (`var list`, `status`, …) keep
  working for agents.
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
