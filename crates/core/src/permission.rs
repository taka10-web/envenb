//! The AI Permission Engine.
//!
//! Input: who (client), where (project / environment / connection), what (action).
//! Output: ALLOW / ASK / DENY. Only EnvFish's own rules are consulted — never text
//! supplied by the AI client.
//!
//! Resolution: among stored rules whose scope matches (a `None` field matches
//! anything), the most specific wins (more non-`None` fields = more specific; ties
//! broken by newest). With no matching rule, built-in defaults apply:
//!
//! | environment          | READ  | WRITE | DELETE |
//! |----------------------|-------|-------|--------|
//! | production-like      | ASK   | DENY  | DENY   |
//! | everything else      | ALLOW | ASK   | DENY   |

use crate::model::{Action, Decision, Permission};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope<'a> {
    pub client_id: Option<&'a str>,
    pub project_id: Option<&'a str>,
    pub environment_id: Option<&'a str>,
    pub connection_id: Option<&'a str>,
}

/// Heuristic used only for defaults; explicit rules always win.
pub fn is_production_like(environment_name: &str) -> bool {
    let n = environment_name.to_ascii_lowercase();
    n.contains("prod") || n == "live" || n == "release"
}

pub fn default_decision(environment_name: &str, action: Action) -> Decision {
    match (is_production_like(environment_name), action) {
        (true, Action::Read) => Decision::Ask,
        (true, _) => Decision::Deny,
        (false, Action::Read) => Decision::Allow,
        (false, Action::Write) => Decision::Ask,
        (false, Action::Delete) => Decision::Deny,
    }
}

fn matches(rule_field: &Option<String>, actual: Option<&str>) -> bool {
    match rule_field {
        None => true,
        Some(v) => actual == Some(v.as_str()),
    }
}

fn specificity(rule: &Permission) -> usize {
    [
        &rule.client_id,
        &rule.project_id,
        &rule.environment_id,
        &rule.connection_id,
    ]
    .iter()
    .filter(|f| f.is_some())
    .count()
}

/// Pick the decision for `action` in `scope` from `rules` (any order), or fall back
/// to the defaults for `environment_name`.
pub fn decide(rules: &[Permission], scope: &Scope<'_>, action: Action, environment_name: &str) -> Decision {
    rules
        .iter()
        .filter(|r| r.action == action)
        .filter(|r| {
            matches(&r.client_id, scope.client_id)
                && matches(&r.project_id, scope.project_id)
                && matches(&r.environment_id, scope.environment_id)
                && matches(&r.connection_id, scope.connection_id)
        })
        .max_by(|a, b| {
            specificity(a)
                .cmp(&specificity(b))
                .then(a.updated_at.cmp(&b.updated_at))
        })
        .map(|r| r.decision)
        .unwrap_or_else(|| default_decision(environment_name, action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn rule(
        client: Option<&str>,
        env: Option<&str>,
        conn: Option<&str>,
        action: Action,
        d: Decision,
    ) -> Permission {
        Permission {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: client.map(String::from),
            project_id: None,
            environment_id: env.map(String::from),
            connection_id: conn.map(String::from),
            action,
            decision: d,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn defaults_follow_the_design_table() {
        assert_eq!(default_decision("development", Action::Read), Decision::Allow);
        assert_eq!(default_decision("development", Action::Write), Decision::Ask);
        assert_eq!(default_decision("development", Action::Delete), Decision::Deny);
        assert_eq!(default_decision("production", Action::Read), Decision::Ask);
        assert_eq!(default_decision("Production", Action::Write), Decision::Deny);
        assert_eq!(default_decision("prod-eu", Action::Delete), Decision::Deny);
    }

    #[test]
    fn most_specific_rule_wins() {
        let rules = vec![
            rule(None, None, None, Action::Write, Decision::Deny),
            rule(Some("claude"), None, None, Action::Write, Decision::Ask),
            rule(
                Some("claude"),
                Some("env-dev"),
                Some("supabase"),
                Action::Write,
                Decision::Allow,
            ),
        ];
        let scope = Scope {
            client_id: Some("claude"),
            project_id: Some("p"),
            environment_id: Some("env-dev"),
            connection_id: Some("supabase"),
        };
        assert_eq!(
            decide(&rules, &scope, Action::Write, "development"),
            Decision::Allow
        );

        let other_conn = Scope {
            connection_id: Some("aws"),
            ..scope.clone()
        };
        assert_eq!(
            decide(&rules, &other_conn, Action::Write, "development"),
            Decision::Ask
        );

        let other_client = Scope {
            client_id: Some("codex"),
            ..scope.clone()
        };
        assert_eq!(
            decide(&rules, &other_client, Action::Write, "development"),
            Decision::Deny
        );
        // Unrelated action falls back to defaults.
        assert_eq!(decide(&rules, &scope, Action::Read, "production"), Decision::Ask);
    }
}
