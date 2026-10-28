# DeepTutorShell

把 [DeepTutor](https://github.com/HKUDS/DeepTutor) 网页应用包成**双击即用的桌面软件**——独立应用窗口（非浏览器）、自动拉起后端服务、关窗即走，形态类似 GitHub Desktop。

> ⚠️ **非官方声明**：本项目是社区个人作品，与 DeepTutor 官方（HKUDS）**无任何关联**，亦不代表其立场。DeepTutor 本身以 Apache-2.0 协议发布，版权归其作者所有；本项目仅通过本地 HTTP 调用与其交互，**不包含、不修改其任何代码**。

## 功能

- **双击启动**：自动检测 `127.0.0.1:3782`（前端）/`8001`（后端）——服务已在跑则直接附着进界面，没在跑则后台拉起（`deeptutor start --no-browser`，隐藏控制台）
- **启动过渡页**：内置加载页 + 进度提示；60 秒未就绪显示排障面板（日志/重试）
- **独立窗口**：webview 直接导航到本地服务，无地址栏、任务栏独立图标；导航白名单限本机端口，外链自动转系统浏览器
- **关窗即走**：关闭窗口 = 退出并停止 DeepTutor 服务（进程树清理 + 端口扫尾，`netstat` 立即干净）
- **嵌入服务联动（v1.1.0.0）**：自动拉起本地 BGE-M3 嵌入服务（知识库检索依赖，默认端口 6066），关窗一并停止；嵌入服务未部署时自动跳过
- **单实例**：重复双击只唤起已有窗口
- **便携配置**：exe 旁可放 `config.json` 覆盖 CLI 路径/工作区/端口，缺省用默认值

## 开发与构建

```bash
npm install
npm run tauri dev     # 开发（debug exe 双击不可用，必须走 dev）
npm run tauri build   # 打包：NSIS 安装版 + MSI + 绿色版（exe 与 WebView2Loader.dll 同目录）
```

环境：Node.js + Rust（**windows-gnu / MinGW 工具链**）。

已知 MinGW 坑（本仓库已配好）：

- `Cargo.toml` 的 `crate-type` 只留 `["rlib"]`（cdylib 会报 export ordinal too large）
- 单测必须 `cargo test --release`（debug 测试二进制起不来）
- `src-tauri/libs/WebView2Loader.dll` 经 `bundle.resources` 随包分发，绿色版必须与 exe 同目录

## 配置（可选）

exe 旁 `config.json`：

```json
{
  "deeptutor_exe": "E:\\DeepTutor\\.venv\\Scripts\\deeptutor.exe",
  "workspace": "E:\\DeepTutor\\dt-home",
  "frontend_port": 3782,
  "backend_port": 8001,
  "embedding_server": {
    "python": "E:\\DeepTutor\\.venv\\Scripts\\python.exe",
    "script": "E:\\BGE-M3\\serve_bge_m3.py",
    "port": 6066,
    "ready_timeout_secs": 150
  }
}
```

## 第三方组件与许可证

| 组件 | 许可证 | 说明 |
| --- | --- | --- |
| 本仓库自研代码 | MIT（见 LICENSE） | 桌面壳、启动/探测逻辑 |
| DeepTutor（HKUDS） | Apache-2.0 | **不在本仓库内**；运行时按用户本机安装的副本经 HTTP 调用 |
| WebView2Loader.dll | Microsoft WebView2 SDK 再分发条款 | 随应用分发（Tauri/MinGW 工具链所需），微软许可允许 |
| Tauri / React / Vite 等依赖 | MIT / Apache-2.0 | 经 npm / crates.io 引用，不随仓库分发 |

使用本软件即表示理解：DeepTutor、MinerU 等周边工具需自行安装并遵守其各自许可证；本软件不对第三方服务的可用性负责。

## License

[MIT](LICENSE)
