use std::sync::Arc;

use envenb_daemon::{Agent, AgentRequest, AgentResponse, socket_path};

use crate::commands::Ctx;
use crate::i18n::tr;

pub async fn run(ctx: Ctx, ping: bool) -> anyhow::Result<()> {
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
            if !ctx.json {
                eprintln!(
                    "{} {}",
                    tr("EnvEnb agent listening on", "EnvEnb Agent を待ち受け中:"),
                    path.display()
                );
            }
            let agent = Arc::new(Agent::new(Arc::new(ctx.app)));
            envenb_daemon::uds::serve(agent, listener).await?;
            Ok(())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (ping, path);
        anyhow::bail!(
            "{}",
            tr(
                "the Local Agent socket is only available on Unix for now",
                "Local Agent のソケットは現時点で Unix のみ対応です"
            )
        )
    }
}
