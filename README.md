# Ly Desktop

适用于 Windows 的 Tauri + React 桌面模板。首次启动进入项目列表，创建或导入项目后可在同一窗口切换。应用提供浅色、深色和系统主题，设置与项目使用 JSON 持久化。MCP、授权服务和更新源均可在接入具体产品时配置。

## 本地启动

准备 Windows x64、MSVC C++ 生成工具、Windows SDK、WebView2 Runtime、Node.js 26.13.1、pnpm 12.21.0 和 rustup。Rust 版本由仓库工具链文件指定。在仓库根目录运行：

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev:web` 仅运行浏览器预览。调试与正式版分别在当前用户、当前应用标识对应的本地数据目录中使用 development 或 production 子目录，实际路径见“关于”。同一目录只允许一个桌面实例运行。

## 验证与打包

```powershell
pnpm test
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml --features custom-protocol
pnpm package
```

Windows 安装包输出到 `src-tauri/target/release/bundle/nsis/`。构建 Rust 测试的内嵌前端资源前，先运行 `pnpm build`。

## 派生新产品

- [初始化名称、应用标识与图标](docs/initialization.md)。初始化后在 [modules.tsx](src/app/modules.tsx) 注册产品页面。
- [项目管理与迁移](docs/projects.md)。导入、复制均创建新 ID，业务模块通过项目目录保存可迁移的资源。
- [软件授权](docs/licensing.md)。配置外部 Grantmint 服务后可激活和续期，授权状态通过非阻塞提示呈现。
- [本机 MCP](docs/mcp.md)。默认关闭，设置页可启停和设置端口。空模板的工具清单为空。
- [在线更新](docs/updating.md)。更新由产品配置，后台检查，用户决定何时安装。

文本日志按本地日期或 10 MiB 分卷。在“关于”中可用系统默认编辑器打开当前文件。[runtime.json](src-tauri/runtime.json) 控制归档保留天数，默认 180，范围 1–365。

[框架契约](docs/framework-proposal.md) · [依赖版本](docs/dependencies.md)
