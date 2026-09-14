-- EnvEnb Phase 1 schema.
-- Secrets live in their own table which has NO plaintext column by design.

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    local_path  TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE environments (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL COLLATE NOCASE,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (project_id, name)
);

-- PUBLIC variables: values an AI agent is allowed to see.
CREATE TABLE variables (
    id              TEXT PRIMARY KEY,
    environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    value           TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (environment_id, name)
);

-- SECRET variables: only ciphertext + nonce are stored.
-- Associated data for the AEAD is the row id, so a ciphertext cannot be
-- re-attached to another row without failing authentication.
CREATE TABLE secrets (
    id              TEXT PRIMARY KEY,
    environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    ciphertext      BLOB NOT NULL,
    nonce           BLOB NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (environment_id, name)
);

CREATE INDEX idx_environments_project ON environments(project_id);
CREATE INDEX idx_variables_env ON variables(environment_id);
CREATE INDEX idx_secrets_env ON secrets(environment_id);
