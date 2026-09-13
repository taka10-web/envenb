-- Phase 2: settings, connections, AI clients, permissions, audit log, approvals.
-- Rule unchanged: no table other than `secrets` ever holds a secret value, and
-- `secrets` only holds ciphertext.

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- External service connection. Credentials are *references* to SECRET variables
-- in the same environment (by name), never values.
CREATE TABLE connections (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,              -- generic_http | openai | supabase
    name            TEXT NOT NULL,
    base_url        TEXT NOT NULL,
    auth_secret     TEXT,                       -- name of the SECRET variable holding the credential
    auth_style      TEXT NOT NULL,              -- bearer | header:<Name> | query:<name> | none | supabase
    metadata        TEXT NOT NULL DEFAULT '{}', -- JSON, non-secret (e.g. extra headers, allowed paths)
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (environment_id, name)
);

CREATE TABLE ai_clients (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE COLLATE NOCASE,
    kind          TEXT NOT NULL,                -- claude_code | codex | mcp | other
    created_at    TEXT NOT NULL,
    last_seen_at  TEXT
);

-- Permission rule. NULL in a scope column means "any". Most specific rule wins;
-- absence of a rule falls back to built-in defaults (see core::permission).
CREATE TABLE permissions (
    id              TEXT PRIMARY KEY,
    client_id       TEXT REFERENCES ai_clients(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE CASCADE,
    environment_id  TEXT REFERENCES environments(id) ON DELETE CASCADE,
    connection_id   TEXT REFERENCES connections(id) ON DELETE CASCADE,
    action          TEXT NOT NULL,              -- READ | WRITE | DELETE
    decision        TEXT NOT NULL,              -- ALLOW | ASK | DENY
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_permissions_lookup ON permissions(client_id, project_id, environment_id, connection_id, action);

-- Pending ASK decisions. The MCP server inserts, a human resolves from Desktop/CLI.
CREATE TABLE approvals (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL,
    client_name     TEXT NOT NULL,
    project_id      TEXT,
    environment_id  TEXT,
    connection_id   TEXT,
    action          TEXT NOT NULL,
    summary         TEXT NOT NULL,              -- human-readable, no secrets
    status          TEXT NOT NULL,              -- PENDING | APPROVED | DENIED | EXPIRED
    created_at      TEXT NOT NULL,
    resolved_at     TEXT,
    expires_at      TEXT NOT NULL
);

CREATE INDEX idx_approvals_status ON approvals(status, created_at);

CREATE TABLE audit_logs (
    id              TEXT PRIMARY KEY,
    client_id       TEXT,
    client_name     TEXT NOT NULL,
    project_id      TEXT,
    project_name    TEXT,
    environment_id  TEXT,
    environment_name TEXT,
    connection_id   TEXT,
    connection_name TEXT,
    action          TEXT NOT NULL,
    summary         TEXT NOT NULL,              -- e.g. "GET /rest/v1/beans" — never a secret
    decision        TEXT NOT NULL,              -- ALLOWED | DENIED | ASKED | ERROR
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);
