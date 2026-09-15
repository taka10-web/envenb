//! Session issuing over the management socket.
//!
//! The socket is how the CLI, MCP and the desktop app ask for a proxy session.
//! What matters here is that a session is a capability for *EnvEnb* and never
//! carries a provider credential, and that it is refused when there is no
//! proxy to use it against.

use std::sync::Arc;

use envenb_core::vault::InMemoryMasterKeyProvider;
use envenb_core::{EnvEnb, NewConnection, SecretValue};
use envenb_daemon::{Agent, AgentRequest, AgentResponse};

const REAL_SECRET: &str = "sk-DAEMON-REAL-SECRET";

async fn core() -> (Arc<EnvEnb>, String, String) {
    let provider = InMemoryMasterKeyProvider::random();
    let app = EnvEnb::open_in_memory(&provider).await.unwrap();
    let p = app.create_project("my-app", None).await.unwrap();
    let e = app.create_environment(&p.id, "development").await.unwrap();
    app.set_secret_variable(&e.id, "API_KEY", SecretValue::new(REAL_SECRET))
        .await
        .unwrap();
    app.create_connection(NewConnection {
        environment_id: e.id.clone(),
        kind: envenb_core::ConnectionKind::GenericHttp,
        name: "provider".into(),
        base_url: Some("https://example.test".into()),
        auth_secret: Some("API_KEY".into()),
        auth_style: Some("bearer".into()),
        metadata: None,
    })
    .await
    .unwrap();
    (Arc::new(app), p.id, e.id)
}

fn issue(project_id: &str, environment_id: &str) -> AgentRequest {
    AgentRequest::IssueSession {
        project_id: project_id.to_string(),
        environment_id: environment_id.to_string(),
        connections: vec!["provider".into()],
        client: "test".into(),
        ttl_seconds: 3600,
    }
}

#[tokio::test]
async fn a_session_carries_a_token_and_a_url_but_never_a_secret() {
    let (app, project_id, environment_id) = core().await;
    let sessions = envenb_core::session::SessionStore::new();
    let agent = Agent::new(app).with_proxy(sessions.clone(), "http://127.0.0.1:7878".into());

    let response = agent.handle(issue(&project_id, &environment_id)).await;
    let AgentResponse::Session {
        token,
        proxy_url,
        expires_in_seconds,
    } = response
    else {
        panic!("expected a session, got {response:?}");
    };

    assert_eq!(proxy_url, "http://127.0.0.1:7878");
    assert_eq!(expires_in_seconds, 3600);
    assert_ne!(token, REAL_SECRET);
    assert!(
        !format!("{token}{proxy_url}").contains(REAL_SECRET),
        "a session response must not carry the provider credential"
    );

    // The token works, and is scoped to the connection it was asked for.
    let scope = sessions.lookup(&token).expect("token should be live");
    assert_eq!(scope.environment_id, environment_id);
    assert!(scope.allows_connection("provider"));
    assert!(!scope.allows_connection("something-else"));
}

#[tokio::test]
async fn a_session_is_refused_when_no_proxy_is_running() {
    let (app, project_id, environment_id) = core().await;
    // An agent built without `with_proxy` has nowhere to send the caller.
    let agent = Agent::new(app);

    let response = agent.handle(issue(&project_id, &environment_id)).await;
    assert!(
        matches!(response, AgentResponse::Error { .. }),
        "expected an error, got {response:?}"
    );
}

#[tokio::test]
async fn an_unknown_environment_does_not_mint_a_token() {
    let (app, project_id, _) = core().await;
    let sessions = envenb_core::session::SessionStore::new();
    let agent = Agent::new(app).with_proxy(sessions.clone(), "http://127.0.0.1:7878".into());

    let response = agent.handle(issue(&project_id, "no-such-environment")).await;
    assert!(
        matches!(response, AgentResponse::Error { .. }),
        "expected an error, got {response:?}"
    );
    assert!(sessions.is_empty(), "no token should have been minted");
}

#[tokio::test]
async fn revoking_stops_a_token_working() {
    let (app, project_id, environment_id) = core().await;
    let sessions = envenb_core::session::SessionStore::new();
    let agent = Agent::new(app).with_proxy(sessions.clone(), "http://127.0.0.1:7878".into());

    let AgentResponse::Session { token, .. } = agent.handle(issue(&project_id, &environment_id)).await
    else {
        panic!("expected a session");
    };
    assert!(sessions.lookup(&token).is_some());

    let response = agent
        .handle(AgentRequest::RevokeSession {
            token: token.clone(),
        })
        .await;
    assert!(matches!(response, AgentResponse::Ok), "{response:?}");
    assert!(sessions.lookup(&token).is_none());
}
