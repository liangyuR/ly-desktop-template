# MCP 接入

在设置页启用后，客户端通过 Streamable HTTP 连接显示的地址，例如 http://127.0.0.1:8765/mcp。首次启动默认关闭。框架绑定 IPv4 回环，端口成功启用后才保存设置。重新启动应用会恢复上次选择。关闭服务释放监听端口，换用被占用的端口会保留原有服务与设置。

## 注册业务工具

在派生应用的 [mcp.rs](../src-tauri/src/mcp.rs) 中为 AppTools 声明 rmcp 工具路由，再从 product_tools 返回该路由。以下示例为业务注册提供入口：

```rust
#[rmcp::tool_router(router = application_routes)]
impl AppTools {
    #[rmcp::tool(description = "读取本产品版本")]
    fn product_version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }
}

fn product_tools() -> ToolRouter<AppTools> {
    AppTools::application_routes()
}
```

业务模块决定工具的参数、操作权限与副作用。显式注册的工具可被同一台设备上的本地客户端发现和调用，应用应在公开写操作前自行核对权限。桌面服务只接受对应本机地址的 Host；原生客户端可省略 Origin，携带 Origin 的浏览器请求须来自该 MCP 服务自己的本机来源。
