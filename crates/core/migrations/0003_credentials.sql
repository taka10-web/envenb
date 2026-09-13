-- Credentials: structured secrets (accounts, SSH, databases, files/certificates).
-- Every field value is stored sealed; metadata columns hold only non-secret
-- identifiers. Nothing here has a plaintext column.

CREATE TABLE credentials (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,              -- account | ssh | database | file
    name            TEXT NOT NULL,
    note            TEXT,                       -- non-secret memo (never a value)
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (environment_id, name)
);

-- One row per field. `secret = 1` fields are sealed (ciphertext + nonce, AAD = row id);
-- `secret = 0` fields (host, port, username, url, filename) are stored in `value`.
CREATE TABLE credential_fields (
    id              TEXT PRIMARY KEY,
    credential_id   TEXT NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    field           TEXT NOT NULL,              -- e.g. username | password | private_key | content
    secret          INTEGER NOT NULL,           -- 0 | 1
    value           TEXT,                       -- non-secret fields only
    ciphertext      BLOB,                       -- secret fields only
    nonce           BLOB,
    updated_at      TEXT NOT NULL,
    UNIQUE (credential_id, field),
    CHECK ((secret = 1 AND value IS NULL) OR (secret = 0 AND ciphertext IS NULL AND nonce IS NULL))
);

CREATE INDEX idx_credentials_env ON credentials(environment_id);
