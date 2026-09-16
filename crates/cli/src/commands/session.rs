//! `envenb session`: the environment an application needs to reach the proxy.
//!
//! This prints a session token and a base URL — never a provider secret. The
//! application points its SDK at the URL and passes the token where the API key
//! used to go; EnvEnb attaches the real credential on the way out.

use envenb_daemon::{AgentRequest, AgentResponse, socket_path};

use crate::commands::Ctx;
use crate::i18n::tr;

pub async fn run(ctx: &Ctx, connections: Vec<String>, ttl: u64, client: String) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    let path = socket_path(ctx.app.paths().root());

    let response = envenb_daemon::uds::request(
        &path,
        &AgentRequest::IssueSession {
            project_id: project.id.clone(),
            environment_id: env.id.clone(),
            connections: connections.clone(),
            client,
            ttl_seconds: ttl,
        },
    )
    .await
    .map_err(|e| {
        anyhow::anyhow!(
            "{}: {e}. {}",
            tr(
                "could not reach the EnvEnb agent",
                "EnvEnb Agent に接続できません"
            ),
            tr(
                "Start it with `envenb agent`.",
                "`envenb agent` で起動してください。"
            )
        )
    })?;

    let (token, proxy_url, expires_in_seconds) = match response {
        AgentResponse::Session {
            token,
            proxy_url,
            expires_in_seconds,
        } => (token, proxy_url, expires_in_seconds),
        AgentResponse::Error { message } => anyhow::bail!("{message}"),
        other => anyhow::bail!("unexpected response: {other:?}"),
    };

    if ctx.json {
        return crate::output::print_json(&serde_json::json!({
            "token": token,
            "proxy_url": proxy_url,
            "expires_in_seconds": expires_in_seconds,
            "project": project.name,
            "environment": env.name,
            "connections": connections,
        }));
    }

    // Shell-evaluable so `eval "$(envenb session)"` just works, with the
    // per-connection base URLs listed as comments for SDK configuration.
    println!("export ENVENB_PROXY_URL={proxy_url}");
    println!("export ENVENB_SESSION_TOKEN={token}");
    let conns = ctx.app.list_connections(&project.id).await.unwrap_or_default();
    let visible: Vec<_> = conns
        .iter()
        .filter(|c| c.environment_id == env.id)
        .filter(|c| connections.is_empty() || connections.contains(&c.name))
        .collect();
    if !visible.is_empty() {
        println!();
        println!(
            "# {}",
            tr(
                "Point each SDK's base URL at its connection:",
                "各 SDK の baseURL は接続ごとに次を指定します:"
            )
        );
        for c in visible {
            println!("#   {}: {proxy_url}/{}", c.name, c.name);
        }
    }
    println!();
    println!(
        "# {}",
        tr(
            "Pass the token where the SDK expects its API key. It is not a provider key",
            "SDK の apiKey にはこのトークンを渡します。Provider の鍵ではありません"
        )
    );
    println!("# {} {expires_in_seconds}s", tr("Valid for", "有効期間:"));
    Ok(())
}
