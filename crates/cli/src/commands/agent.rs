use std::sync::Arc;

use envenb_daemon::{Agent, AgentRequest, AgentResponse, socket_path};

use crate::commands::Ctx;
use crate::i18n::tr;

pub async fn run(ctx: Ctx, ping: bool, proxy_port: u16) -> anyhow::Result<()> {
    let path = socket_path(ctx.app.paths().root());

    #[cfg(unix)]
    {
        if ping {
            let resp = envenb_daemon::uds::request(&path, &AgentRequest::Ping).await?;
            match resp {
                AgentResponse::Pong { version } => {
                    if ctx.json {
                        println!("{}", serde_json::json!({ "pong": true, "version": version }));
                    } else {
                        println!(
                            "{} envenb-agent {version} ({})",
                            tr("pong from", "応答あり:"),
                            path.display()
                        );
                    }
                    Ok(())
                }
                other => anyhow::bail!("unexpected response: {other:?}"),
            }
        } else {
            let listener = envenb_daemon::uds::bind(&path)?;
            let core = Arc::new(ctx.app);

            // The HTTP proxy is what applications and SDKs talk to; the socket
            // above stays for management traffic (CLI, MCP, the desktop app).
            let sessions = envenb_core::session::SessionStore::new();
            let proxy = envenb_broker::proxy::serve(core.clone(), sessions.clone(), proxy_port).await?;
            let proxy_url = proxy.base_url();

            if !ctx.json {
                eprintln!(
                    "{} {}",
                    tr("EnvEnb agent listening on", "EnvEnb Agent を待ち受け中:"),
                    path.display()
                );
                eprintln!(
                    "{} {proxy_url}/<connection>",
                    tr("HTTP proxy for SDKs on", "SDK 向け HTTP Proxy:")
                );
            }

            let agent = Arc::new(Agent::new(core).with_proxy(sessions, proxy_url));
            envenb_daemon::uds::serve(agent, listener).await?;
            Ok(())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (ping, path, proxy_port);
        anyhow::bail!(
            "{}",
            tr(
                "the Local Agent socket is only available on Unix for now",
                "Local Agent のソケットは現時点で Unix のみ対応です"
            )
        )
    }
}
