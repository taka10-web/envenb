//! Short-lived capabilities for the local HTTP proxy.
//!
//! A session token authenticates a *client of EnvEnb* — it is not a provider
//! credential and grants no access to one. It says only: "this process may ask
//! EnvEnb to call these connections, in this environment, until this time."
//!
//! Tokens live in memory, never in the vault, and only their hash is kept, so a
//! heap dump of the daemon does not hand out usable tokens.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::RngCore;
use sha2::{Digest, Sha256};

/// How long a freshly minted token stays valid.
pub const DEFAULT_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// What a token is allowed to do. Everything outside this is denied.
#[derive(Debug, Clone)]
pub struct SessionScope {
    pub project_id: String,
    pub environment_id: String,
    /// Connection names this token may reach. Empty means every connection in
    /// the environment.
    pub connections: Vec<String>,
    /// Name recorded in the audit log for calls made with this token.
    pub client: String,
}

impl SessionScope {
    /// Whether this token may use `connection`.
    pub fn allows_connection(&self, connection: &str) -> bool {
        self.connections.is_empty() || self.connections.iter().any(|c| c == connection)
    }
}

struct Entry {
    scope: SessionScope,
    expires_at: Instant,
}

/// The daemon's live token table.
#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<[u8; 32], Entry>>>,
}

/// A token as handed to the client. Printing it is the caller's decision; it is
/// never logged by this module.
pub struct IssuedToken {
    pub token: String,
    pub expires_in: Duration,
}

fn hash(token: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    h.finalize().into()
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a token for `scope`. Only its hash is retained.
    pub fn issue(&self, scope: SessionScope, ttl: Duration) -> IssuedToken {
        let mut raw = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut raw);
        let token = hex(&raw);
        let mut map = self.inner.lock().expect("session store poisoned");
        map.retain(|_, e| e.expires_at > Instant::now());
        map.insert(
            hash(&token),
            Entry {
                scope,
                expires_at: Instant::now() + ttl,
            },
        );
        IssuedToken {
            token,
            expires_in: ttl,
        }
    }

    /// The scope `token` carries, or `None` when unknown or expired.
    pub fn lookup(&self, token: &str) -> Option<SessionScope> {
        let mut map = self.inner.lock().expect("session store poisoned");
        let key = hash(token);
        match map.get(&key) {
            Some(e) if e.expires_at > Instant::now() => Some(e.scope.clone()),
            Some(_) => {
                map.remove(&key);
                None
            }
            None => None,
        }
    }

    /// Invalidate a token immediately.
    pub fn revoke(&self, token: &str) {
        self.inner
            .lock()
            .expect("session store poisoned")
            .remove(&hash(token));
    }

    pub fn len(&self) -> usize {
        let mut map = self.inner.lock().expect("session store poisoned");
        map.retain(|_, e| e.expires_at > Instant::now());
        map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> SessionScope {
        SessionScope {
            project_id: "p".into(),
            environment_id: "e".into(),
            connections: vec!["gemini".into()],
            client: "test".into(),
        }
    }

    #[test]
    fn a_token_is_accepted_once_issued_and_not_before() {
        let store = SessionStore::new();
        let issued = store.issue(scope(), DEFAULT_TTL);
        assert!(store.lookup(&issued.token).is_some());
        assert!(store.lookup("not-a-real-token").is_none());
    }

    #[test]
    fn an_expired_token_stops_working() {
        let store = SessionStore::new();
        let issued = store.issue(scope(), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(20));
        assert!(store.lookup(&issued.token).is_none());
        assert!(store.is_empty(), "expired entries must not accumulate");
    }

    #[test]
    fn revoking_takes_effect_immediately() {
        let store = SessionStore::new();
        let issued = store.issue(scope(), DEFAULT_TTL);
        store.revoke(&issued.token);
        assert!(store.lookup(&issued.token).is_none());
    }

    #[test]
    fn the_raw_token_is_not_retained() {
        let store = SessionStore::new();
        let issued = store.issue(scope(), DEFAULT_TTL);
        let map = store.inner.lock().unwrap();
        assert!(
            !map.keys().any(|k| hex(k) == issued.token),
            "the table must hold hashes, not tokens"
        );
    }

    #[test]
    fn a_scope_limits_which_connections_are_reachable() {
        let s = scope();
        assert!(s.allows_connection("gemini"));
        assert!(!s.allows_connection("openai"));

        let any = SessionScope {
            connections: vec![],
            ..scope()
        };
        assert!(any.allows_connection("anything"));
    }
}
