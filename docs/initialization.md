# 派生产品初始化

用 GitHub 的 **Use this template** 创建产品仓库，克隆后在仓库根目录运行初始化脚本。每个产品使用独立的反向域名标识，例如 `com.example.inspector`。Tauri 按此标识选择本机应用数据根目录，开发版和正式版分别使用自己的子目录。

```powershell
pnpm install --frozen-lockfile
node scripts/init-product.mjs --name "质量检查" --package qa-inspector --identifier com.example.inspector --icon brand.png --dry-run
node scripts/init-product.mjs --name "质量检查" --package qa-inspector --identifier com.example.inspector --icon brand.png
```

`--dry-run` 列出即将改动的文件。必填参数分别是用户看到的产品名、npm/Cargo 包名和应用标识。`--icon` 接受方形 PNG 并通过 Tauri CLI 生成 Windows 嵌入图标和应用标题栏图标，省略时沿用模板图标。审核 `git diff` 后提交改动。

脚本仅更新根包名、Cargo 包名和对应锁文件记录、Tauri 产品名/主窗口标题/应用标识、HTML 标题与选中的图标。源码中的业务模块、配置和其他依赖保持原状。初始化会在完成预检后暂存修改和原文，写入异常时回滚本次改动。若进程被强制终止并留下 `.ly-product-init/`，先将 `original/` 中的文件与工作区比较，恢复或保留所需改动，再删除这个临时目录并重试。

已投用产品应保留其应用标识。更换标识后应用会从新的数据目录启动；业务项目可通过导出与导入迁移。授权产品 UUID、在线更新源及公钥由产品维护者分别配置，见[授权接入](licensing.md)与[在线更新](updating.md)。
