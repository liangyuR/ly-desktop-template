# ly-desktop-template

面向通用桌面应用的 Tauri + React 模板，计划沿用 GlueSight 的基本 UI 风格，提取配置、日志与桌面生命周期等基础能力。

**当前状态：设计讨论阶段。仓库已启用 GitHub Template，尚未包含可运行应用。**

## 已确定

- Windows 是桌面模板首版唯一的验收平台。
- 首版内置浅色、深色和跟随系统三种主题模式。
- 仓库公开，名称为 `liangyuR/ly-desktop-template`。
- 面向通用桌面应用，保留基本 UI、配置与日志能力。首版内置项目管理，支持在同一应用进程和窗口中切换项目。
- 首版纳入统一软件授权。派生应用按产品接入同一签发服务，客户端支持联网激活与离线使用。具体规则见[软件授权设计](docs/framework-proposal.md#统一软件授权)。
- 参考 GlueSight 的 `main` 提交 [`432119ad`](https://github.com/liangyuR/ly-gluesight/commit/432119ad1e88041ee33cce4b06caf2e6308f5e5a)。

## 设计入口

[框架设计提案](docs/framework-proposal.md) 记录建议范围、抽取边界和待讨论事项。提案中的能力尚未实现，也不代表已经确定的 API。

代码抽取将在范围确定后开展；模板使用独立 Git 历史和应用身份，不携带源项目运行数据。
