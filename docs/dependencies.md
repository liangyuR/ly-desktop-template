# 首版依赖与版本

核对日期为 2026-11-23。模板目前处于设计阶段。以下是面向 Windows 桌面应用的直接依赖基线，范围包括 UI、项目文件、授权客户端、文本日志、本机 MCP 和在线更新。传递依赖在创建应用时由 pnpm 和 Cargo 锁文件记录。

## 工具链

| 工具 | 版本 | 发布来源 |
| --- | --- | --- |
| Node.js | 26.13.1 LTS | [正式发行索引](https://nodejs.org/dist/index.json) |
| pnpm | 12.21.0 | [npm](https://www.npmjs.com/package/pnpm) |
| Rust（Windows MSVC） | 1.100.0 | [stable 发布清单](https://static.rust-lang.org/dist/channel-rust-stable.toml) |

Windows 原生构建还需要 MSVC C++ 生成工具和 Windows SDK，桌面运行使用 WebView2 Runtime。

## 前端、构建与测试

版本取各包在 [npm 官方注册表](https://registry.npmjs.org/) 的正式 `latest`。qing-steel 使用项目发布的 tarball。

| 用途 | 包与版本 |
| --- | --- |
| 界面 | `react` 19.3.1、`react-dom` 19.3.1、[qing-steel 0.11.2 发布包](https://github.com/liangyuR/qing-steel/releases/download/v0.11.2/qing-steel-0.11.2.tgz) |
| 导航、图标 | `react-router-dom` 7.18.6、`lucide-react` 1.78.0 |
| 桌面调用、检查与安装更新 | `@tauri-apps/api` 2.12.5、`@tauri-apps/plugin-updater` 2.14.1 |
| 开发与构建 | `@tauri-apps/cli` 2.12.6、`typescript` 7.0.2、`vite` 8.3.8、`@vitejs/plugin-react` 6.3.0 |
| 类型声明 | `@types/node` 26.6.6、`@types/react` 19.3.0、`@types/react-dom` 19.3.0 |
| 前端回归 | `vitest` 5.1.1、`jsdom` 31.0.1 |
| 用户交互测试 | `@testing-library/dom` 10.4.2、`@testing-library/react` 16.3.3、`@testing-library/user-event` 14.6.13、`@testing-library/jest-dom` 8.0.0 |

## Rust 桌面端

版本取 [crates.io 官方索引](https://index.crates.io/config.json) 中未撤回的最高稳定版本。Tauri 核心、构建工具和插件分别发布，各自使用其稳定版本。

| 用途 | crate 与版本 |
| --- | --- |
| 桌面运行与构建 | `tauri` 2.12.7、`tauri-build` 2.7.6（构建依赖） |
| 在线更新、打开当前日志 | `tauri-plugin-updater` 2.14.1、`tauri-plugin-opener` 2.7.3 |
| 配置与项目清单 | `serde` 1.0.229、`serde_json` 1.0.154、`uuid` 1.29.3 |
| 项目包、完整性校验、文件选择 | `zip` 9.8.1、`sha2` 0.11.1、`rfd` 0.18.0 |
| 运行日志、文本格式、按日期或大小分卷 | `tracing` 0.1.44、`tracing-subscriber` 0.3.23、`flexi_logger` 0.32.0（`trc` 集成） |
| MCP、异步运行时、本机 HTTP | `rmcp` 3.11.0、`tokio` 1.54.1、`axum` 0.8.9 |
| 授权服务请求、Windows 设备身份与 DPAPI | `reqwest` 0.13.5、`windows-sys` 0.61.2 |

## 授权 SDK

`grantmint-offline` 暂无稳定发布。首版沿用 GlueSight 授权客户端已验收的 Git 修订 `7183c40960853a32cb176be2ccb46c7aa75b50fc`。源码清单中的 `0.1.0` 是包内版本。该 SDK 自身的密码学依赖遵循其 Cargo 清单。[SDK 源码清单](https://github.com/liangyuR/grantmint/blob/7183c40960853a32cb176be2ccb46c7aa75b50fc/verifiers/rust/Cargo.toml)

## 本次核验

在临时目录用 npm 严格检查上表前端组合的 peer dependencies 并成功解析锁文件；qing-steel 发布包声明支持 React 和 React DOM `>=18.2.0 <20`。Rust 直接依赖连同固定修订的授权 SDK 已用 Cargo 成功解析锁文件。应用编译、Windows 启动、授权与更新流程的验证留到模板实现阶段。
