# slim 还可精简的项

只审计，不删代码。远端 `slim` 头是 `e9569d5ece06649d6e0234c12e236f99fae497db`（2026-11-10 合并 PR #34）。`main` 不得跟着这些建议再删。上游水位仍以 [upstream-sync.md](./upstream-sync.md) 为准。

第一轮已经从 `slim` 去掉 MCP、CLI、automation、headless 和 AUR。那些不要加回来。`meatshell mcp` / `meatshell cli` 的非零退出存根要留着。

另两路已经在做，本页不算新建议，也不改它们的文件：只留简约壁纸；以后默认发版只做 Windows 和 macOS ARM。侧栏复用和标签关闭也在另一路。

依赖数用 `cargo +stable tree -e normal --prefix none --offline` 在该头上数过，去重后 574。没有重新做 release 构建，所以下面凡是安装包体积或 CPU 写「未知」的，都还没测过。

## 可以直接清的残留

这些不进安装包，删了也不减少唤醒。验收是仓库里不再被引用，SSH、终端、SFTP、串口和导入导出测试仍通过。

| 残留 | 证据 | 安装包或 CPU |
| --- | --- | --- |
| `output/covers/bilibili-cli-mcp-cover.png` | 1,606,679 字节。除水位表里一句「fork 历史里没有该文件」外，没有生产引用。 | 安装包 0，CPU 未知且预期无变化 |
| `docs/QR/QQ_Group_QR_Code.jpg` | 125,820 字节。README、界面和 Rust 都没有引用。 | 同上 |
| `assets/make_icon.py` | 6,002 字节。发布流程不调用它。 | 同上 |

## 需要用户拍板的真实功能

删掉会少一种用法。体积和 CPU 节省本轮都是未知，不能把源码行数当成收益。

| 功能 | 证据 | 对核心路径 | 怎么验证 |
| --- | --- | --- | --- |
| 彩色 Emoji | `twemoji-assets` 只被 `meatshell` 直接依赖。`src/terminal/impls/presentation.rs` 用它画彩色图。 | 终端仍在，emoji 变回单色。SSH/SFTP/串口不受影响。 | 构建前后比 macOS 或 Windows 包大小；打开含 emoji 的本地终端。 |
| macOS 可选 Skia | `Cargo.toml` 只在 macOS 上给 slint 加 `renderer-skia`。默认仍是 femtovg。 | 不能再用 `SLINT_BACKEND=winit-skia`。 | 同一提交编两次，比 `.app` 大小；默认启动仍可用。 |
| 跟随系统深浅色 | `dark-light` 只被本包直接依赖，`terminal_ui.rs` 在启动时调用。 | 首次主题不再跟系统。 | 删依赖后深色/浅色手动切换仍在。 |
| 设置里选系统字体 | `src/app/fonts.rs` 使用 `fontdb`。 | 只影响字体选择，不改终端协议。 | 设置页仍能用内置字体。 |
| RDP 启动器 | `src/rdp.rs` 调系统客户端，不实现协议。 | 不影响 SSH/SFTP/串口。 | 没有 RDP 会话时，其他连接不受影响。 |
| Telnet | `src/terminal/impls/telnet.rs`。明文协议。 | 少一种会话。SSH 不受影响。 | 串口和 SSH 仍能连。 |
| ZMODEM | `src/terminal/impls/zmodem.rs`。 | 终端里的 sz/rz 消失。SFTP 仍在。 | SFTP 上传下载仍通过。 |
| WebDAV「信任无效证书」 | `src/app/webdav.rs` 的 `accept_invalid_certs`。同步本身继续用 `ureq` 的 TLS。 | 自签 NAS 会失败。不要为了这一个开关删掉 `ureq`。 | 正规 HTTPS 的上传下载仍要口令。 |
| `--config-info` | `src/main.rs` 打印版本、数据目录和会话数。 | 无依赖。 | 没有这个参数时 GUI 仍启动。 |

进程采样在资源监控打开时约每 1.5 秒跑一次远端 `ps`（`AuxiliaryChannelKind::Processes`）。侧栏折叠或专注模式会关掉监控。这是唤醒候选，不是已证实的 CPU 优化。本机 1 秒定时器的说明在交接记录里，这里不重复测。

## 必须保留

- SSH、终端、SFTP、串口，以及 FinalShell / 便携导出导入。`des`/`md5` 只为 FinalShell 导入，`argon2`/`chacha20poly1305` 用于导出和 PPK。
- `ureq` 的 TLS。更新检查和 WebDAV 都用它。修 slim 更新通道时也不能改成不校验证书。
- HTTP 与 SOCKS 代理实现。`https://` 代理的明文问题另案处理，不能靠删掉代理来躲。
- 配置不要加 `deny_unknown_fields`。`slim` 保存不能把 `main` 上显式关掉的 MCP 开关弄丢（交接里的 MSA02）。
- `mcp` / `cli` 参数仍要非零退出且不打开窗口。
- 过期配置快照必须继续拒绝覆盖。

## 优先级

1. 清掉上面三份未引用文件。收益只在仓库，不在安装包。
2. 若要减安装包，先让用户决定彩色 Emoji 和 macOS Skia。必须用真实包对比，禁止用行数估计。
3. 其余功能等用户点名再删。发版平台和壁纸不要在这里再做一遍。
