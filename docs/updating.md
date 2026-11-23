# 在线更新

在派生产品的 `src-tauri/updater.json` 设置 `enabled`、HTTPS 更新清单地址 `endpoint` 和 Tauri 更新签名公钥 `publicKey`。默认关闭。配置属于构建输入，每个派生产品管理自己的更新源和签名密钥。

应用启动后在后台检查一次。“关于”页支持手动检查，发现新版本时显示版本和发布说明。只有用户确认安装后才会下载、校验签名并运行安装器。在 Windows 上由安装器完成更新与重启。未配置、服务不可达及校验失败均可继续使用当前版本。

发布时保持产品的 Tauri identifier，递增应用版本。维护者使用 Tauri CLI 的 signer 生成并保存独立更新密钥，构建签名安装包时通过安全环境提供私钥并启用 `bundle.createUpdaterArtifacts`。发布服务提供清单、对应 Windows 架构的 HTTPS 安装包地址和包签名；公钥与签名须来自同一对更新密钥。具体配置及清单格式见 [Tauri 更新插件说明](https://v2.tauri.app/plugin/updater/)。
