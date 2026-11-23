use crate::settings::Preferences;
use axum::Router;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{ServerCapabilities, ServerConfig},
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Serialize;
use std::{net::Ipv4Addr, time::Duration};
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct AppTools {
    router: ToolRouter<Self>,
}

#[rmcp::tool_handler(router = self.router)]
impl ServerHandler for AppTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
}

fn product_tools() -> ToolRouter<AppTools> {
    ToolRouter::new()
}

struct Running {
    port: u16,
    cancel: CancellationToken,
    handle: JoinHandle<()>,
}

impl Running {
    fn serve(listener: TcpListener, port: u16, router: ToolRouter<AppTools>) -> Self {
        let cancel = CancellationToken::new();
        let config = StreamableHttpServerConfig::default()
            .with_allowed_hosts([format!("127.0.0.1:{port}"), format!("localhost:{port}")])
            .with_allowed_origins([
                format!("http://127.0.0.1:{port}"),
                format!("http://localhost:{port}"),
            ])
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_cancellation_token(cancel.child_token());
        let service = StreamableHttpService::<AppTools, LocalSessionManager>::new(
            move || {
                Ok(AppTools {
                    router: router.clone(),
                })
            },
            Default::default(),
            config,
        );
        let app = Router::new().nest_service("/mcp", service);
        let token = cancel.clone();
        let handle = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app)
                .with_graceful_shutdown(token.cancelled_owned())
                .await
            {
                tracing::warn!(%error, "本机 MCP 服务已停止");
            }
        });
        Self {
            port,
            cancel,
            handle,
        }
    }

    async fn stop(mut self) {
        self.cancel.cancel();
        if tokio::time::timeout(Duration::from_secs(2), &mut self.handle)
            .await
            .is_err()
        {
            self.handle.abort();
            let _ = (&mut self.handle).await;
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.handle.abort();
    }
}

#[derive(Default)]
pub struct Mcp {
    server: Mutex<Option<Running>>,
}

#[derive(Serialize)]
pub struct Status {
    enabled: bool,
    port: u16,
    url: Option<String>,
}

impl Mcp {
    pub async fn restore(&self, prefs: &Preferences) -> Result<(), String> {
        let saved = prefs.get();
        if saved.mcp_enabled {
            self.configure(prefs, true, saved.mcp_port, false).await?;
        }
        Ok(())
    }

    async fn configure(
        &self,
        prefs: &Preferences,
        enabled: bool,
        port: u16,
        persist: bool,
    ) -> Result<Status, String> {
        if port == 0 {
            return Err("MCP 端口范围为 1–65535".into());
        }
        let mut server = self.server.lock().await;
        let live_port = Self::live_port(server.as_ref());
        let requested = enabled.then_some(port);
        let listener = if enabled && live_port != requested {
            Some(
                TcpListener::bind((Ipv4Addr::LOCALHOST, port))
                    .await
                    .map_err(|e| format!("无法监听本机端口 {port}：{e}"))?,
            )
        } else {
            None
        };
        if persist {
            prefs.update(|s| {
                s.mcp_enabled = enabled;
                s.mcp_port = port;
            })?;
        }
        if live_port != requested || !enabled {
            let old = server.take();
            *server = listener.map(|socket| Running::serve(socket, port, product_tools()));
            if let Some(old) = old {
                old.stop().await;
            }
        }
        Ok(Self::status_for(server.as_ref(), port))
    }

    fn live_port(running: Option<&Running>) -> Option<u16> {
        running.filter(|r| !r.handle.is_finished()).map(|r| r.port)
    }

    fn status_for(running: Option<&Running>, configured_port: u16) -> Status {
        let port = Self::live_port(running);
        Status {
            enabled: port.is_some(),
            port: port.unwrap_or(configured_port),
            url: port.map(|p| format!("http://127.0.0.1:{p}/mcp")),
        }
    }
}

#[tauri::command]
pub async fn mcp_status(
    state: tauri::State<'_, Mcp>,
    prefs: tauri::State<'_, Preferences>,
) -> Result<Status, String> {
    let server = state.server.lock().await;
    Ok(Mcp::status_for(server.as_ref(), prefs.get().mcp_port))
}

#[tauri::command]
pub async fn mcp_configure(
    state: tauri::State<'_, Mcp>,
    prefs: tauri::State<'_, Preferences>,
    enabled: bool,
    port: u16,
) -> Result<Status, String> {
    state.configure(&prefs, enabled, port, true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[rmcp::tool_router(router = sample_tools)]
    impl AppTools {
        #[rmcp::tool(description = "Example tool")]
        fn example(&self) -> String {
            "sample-reply".into()
        }
    }

    async fn request(url: &str, method: &str, params: Value) -> Value {
        let _ = rustls::crypto::ring::default_provider().install_default();
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(url)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn empty_template_and_registered_tool_have_distinct_discovery_and_call_results() {
        for (router, registered) in [(product_tools(), false), (AppTools::sample_tools(), true)] {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let running = Running::serve(listener, port, router);
            let url = format!("http://127.0.0.1:{port}/mcp");
            let list = request(&url, "tools/list", json!({})).await;
            let tools = list["result"]["tools"]
                .as_array()
                .expect("tools/list must succeed");
            if registered {
                assert_eq!(tools[0]["name"], "example");
                let call =
                    request(&url, "tools/call", json!({"name":"example","arguments":{}})).await;
                assert_eq!(call["result"]["content"][0]["text"], "sample-reply");
            } else {
                assert!(tools.is_empty());
            }
            running.stop().await;
            assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await.is_ok());
        }
    }

    #[tokio::test]
    async fn port_conflict_preserves_previous_endpoint_and_restart_preference() {
        let dir = tempfile::tempdir().unwrap();
        let prefs = Preferences::open(dir.path()).unwrap();
        let state = Mcp::default();
        let reserved = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let port = reserved.local_addr().unwrap().port();
        drop(reserved);
        let status = state.configure(&prefs, true, port, true).await.unwrap();
        let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let unavailable = occupied.local_addr().unwrap().port();
        assert!(
            state
                .configure(&prefs, true, unavailable, true)
                .await
                .is_err()
        );
        assert_eq!(prefs.get().mcp_port, port);
        let listed = request(status.url.as_ref().unwrap(), "tools/list", json!({})).await;
        assert_eq!(listed["result"]["tools"], json!([]));
        state.configure(&prefs, false, port, true).await.unwrap();
        assert!(!Preferences::open(dir.path()).unwrap().get().mcp_enabled);
        assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await.is_ok());
    }
}
