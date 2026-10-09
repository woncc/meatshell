# MeatShell 精简审计（`slim` 分支）

- 基线：`main` @ `2549ea3`（v0.7.10）
- 日期：2026-10-09
- 范围：仅审计、不改代码。目标是找出可以删掉、让程序更小更安全的功能，MCP 优先。
- 方法：逐模块 `rg` 追引用；`cargo +stable tree --offline -e normal` 统计依赖数（删掉指定直接依赖后重算）；
  对 MCP 做了一次实测构建（Linux `--release --features headless`，`-j3`）。完整 GUI release 构建超过 15 分钟，没有跑。

## 一、结论速览（按优先级）

| 优先级 | 候选 | 体积 / 依赖影响 | 删除风险 | 建议 |
|---|---|---|---|---|
| P0 | **MCP**（stdio + 远程 HTTP/OAuth，`src/mcp/**`） | 实测二进制 **−3.0 MiB**（headless 9.0→6.0 MiB）；Windows 依赖 542→**500**（−42），全平台 783→743；Rust 约 1,070 行 + UI/配置/文档/测试/CI 约 1,300 行 | 低：GUI 不调用；旧配置键可被忽略 | **删** |
| P0 | 仓库杂物 `tools/build_minimax_multishot.py`（ComfyUI 脚本，与产品无关） | 253 行，不进二进制 | 无 | **删** |
| P0 | 与 MCP 绑定的 CI：`remote-mcp.yml`、`verify-published-mcp.yml` | 141 行 YAML | 无（后者只在不存在的分支触发） | **删** |
| P1 | **CLI + automation 层 + `headless` feature**（`src/cli/**`、`src/automation/**`） | 约 1,000 行 Rust + `ssh::execute_command` 约 130 行；无独占 crate；GUI 包内增量估计 <0.5 MiB（未单测） | 中低：仅失去命令行用法和 2 个 e2e 测试脚本 | **建议删**（需你确认是否用 `meatshell cli`） |
| P1 | AUR 打包（`packaging/aur/**` + `aur-publish.yml`） | 161 行 | 无：工作流被门禁锁在上游仓库，fork 永不运行；PKGBUILD 指向上游且 `SKIP` 校验 | **删** |
| P2 | Flatpak 打包（`packaging/flatpak/**` + release.yml 的 `build-flatpak` job） | 120 行 + CI 一个 job | 低：失去 `.flatpak` 产物；app-id 仍是上游 `io.github.yituorou.meatshell` | **可选删** |
| P2 | 内置壁纸 `miku.jpg` | 二进制约 −0.3~0.6 MiB（`include_bytes!` + 设置页缩略图各嵌一次） | 低：选了它的用户回落默认壁纸；且有版权疑虑 | **可选删**（保留默认 `ms.jpg`） |
| P2 | WebDAV「信任自签名/内网证书」开关 | 约 40 行 + `src/webdav/**` 62 行 | 低：自签 NAS 用户需改用正规证书 | **可选删**（或改为证书指纹固定） |
| P2 | Cargo.toml 中未直接使用的依赖 `thiserror`、`russh-keys` | 依赖数 0 变化（均为传递依赖） | 无 | **删**（清理） |
| P3 | `--config-info` 启动参数 | 约 15 行 | 无（无任何调用方） | 可选删 |
| — | 更新检查（GitHub API） | 依赖 0 变化（`ureq` 仍被 WebDAV 需要） | — | 保留（已可关闭，已指向本 fork） |
| — | WebDAV 同步、托盘、单实例 IPC、串口、Telnet、ZMODEM、RDP 启动器、彩色 Emoji、端口转发、触发器、进程/系统信息/编辑器窗口 | 见第三节 | 删除会损失用户功能 | 保留（部分附加固建议） |

**最推荐的组合**：一次性删 MCP + 其 CI/文档/测试 + 杂物脚本 + AUR，再由你决定 CLI 是否一起删。仅这一步就能减少约 3 MiB 二进制、42 个 Windows 依赖 crate，并移除默认全开的远程控制入口（见 H2）。

## 二、P0/P1 候选详情

### 1. MCP 服务（删）

**它做什么。** `meatshell mcp serve` 以 stdio 方式向 AI 客户端暴露工具：列会话、执行远程命令、SFTP 上传下载、导入会话；加 `--http-config` 则在本机监听 HTTP（axum + rmcp streamable HTTP），用外部签发的 OAuth JWT 鉴权，配合反向代理做远程 MCP。

**涉及文件。**

| 位置 | 内容 | 行数 |
|---|---|---|
| `src/mcp/**`（server/tools/http/oauth） | 协议、HTTP 监听、JWT 校验 | 1,066 |
| `src/main.rs` | `mod mcp`、`StartMode::Mcp`、`--http-config` 检查、`headless` 报错文案、单测 | ~20 |
| `src/automation/struct/access.rs` | `Frontend::Mcp` 及权限门（若保留 CLI 需改成只剩 CLI） | 26 |
| `src/automation/impls/tools.rs`、`sftp.rs` | 读取 `mcp_*` 权限、`mcp_access` 过滤（若保留 CLI，需删 MCP 分支） | 部分 |
| `src/config/struct/config_file.rs` | `mcp_enabled / mcp_use_saved_credentials / mcp_allow_commands / mcp_allow_file_transfers` 四个字段 + `default_mcp_preview_enabled()`（**默认 true**） | ~20 |
| `src/config/impls/config.rs:1407-1430` | 8 个 getter/setter | 24 |
| `src/config/struct/session.rs:179-183, 369` | 会话级 `mcp_access` | 6 |
| `src/config/impls/import_tests.rs:160-190` | 导入不覆盖 MCP 设置的测试 | ~30 |
| `src/app/settings_callbacks.rs:58-75` | 设置页联动 | ~18 |
| `src/app/session_callbacks.rs:168,722`、`session_models.rs:538` | 会话编辑框勾选项 | 3 |
| `ui/interface_panel.slint`（属性 418-421、481、侧栏入口 571-573、MCP 页 1728-1810） | 设置 → 界面 → MCP 页面 | ~95 |
| `ui/app.slint`（728-732、974、4895、5485-5553） | 属性与回调转发 | ~15 |
| `ui/session_dialog.slint`（73-74、580、687、1140-1150、1600） | 「允许 MCP 访问此会话」勾选与提示 | ~15 |
| `lang/en/LC_MESSAGES/meatshell.po:649-653` | 2 条翻译 | 6 |
| `tests/ui_session_editor.rs:265`、`tests/app/modal_layers.rs:478` | 布局测试里因 MCP 勾选行多出的像素偏移（删后需改数值） | 2 处 |
| `docs/REMOTE_MCP.md` | 远程部署文档 | 375 |
| `tests/remote_mcp_e2e.py`、`tests/tls_proxy_fixture.py`（MCP 部分）、`tests/config_import_e2e.py`（MCP 部分） | e2e | ~500 |
| `.github/workflows/remote-mcp.yml`、`verify-published-mcp.yml` | MCP/headless CI | 141 |
| `.github/workflows/release.yml` | 5 处 `cp docs/REMOTE_MCP.md` | 5 |
| `Cargo.toml` | deb 资产 `docs/REMOTE_MCP.md`；依赖 `rmcp`、`axum`、`jsonwebtoken`、`tokio-util`、`url`（后两者只有 MCP 用） | ~20 |
| `wix/main.wxs:43-53` | 升级时关闭「MCP 宿主进程」的说明与提示文案 | ~10（文案改成只提 MeatShell 即可，CloseApplication 本身可保留） |
| `README.md` / `README.en.md` 「CLI 与 MCP」一节、特性列表、`assets/architecture-live.gif`（1.2 MB，图里画着 MCP） | 文档 | ~30 + 图 |
| `SECURITY_AUDIT.md` H2、`FORK_NOTES.md` | 引用 | 更新 |

**依赖影响（实测）。**

- Windows 目标 crate 数：542 → **500**；全平台：783 → 743。
- MCP 独占的 crate（Windows）：`rmcp`、`axum`、`axum-core`、`hyper`、`hyper-util`、`http`、`http-body(-util)`、`httparse`、`tower(-layer/-service)`、`matchit`、`jsonwebtoken`、`pem`、`simple_asn1`、`time`(+macros)、`schemars`(+derive)、`sse-stream`、`serde_urlencoded`、`serde_path_to_error`、`rand 0.10`、`chacha20 0.10`、`syn 3` 等 42 个。
- 二进制：Linux headless release，`9,432,480 → 6,294,560` 字节，**−3.0 MiB（−33%）**。GUI 包里这些 crate 同样没有别的使用者，绝对值预计相近（约 −3 MiB），相对比例更小。编译时间也少一大截（同一缓存下增量重编 161 s vs 冷编 381 s，不完全可比，仅供参考）。

**安全意义（最主要的理由）。**

- `default_mcp_preview_enabled()` 返回 **true**：新用户和没写过这些键的旧配置，MCP、「使用已保存凭据」、「执行命令」、「文件传输」全部默认开启；会话级 `mcp_access` 默认也是 true（`SECURITY_AUDIT.md` H2）。任何能启动 `meatshell mcp serve` 的本机进程/AI 客户端，都能用已保存的密码登录所有服务器执行任意命令。
- HTTP 模式是一个网络监听器 + JWT 校验 + OAuth 资源服务器，是整个程序最大的远程攻击面。
- #26 修复时专门为 MCP 导入工具补了口令校验；删掉后这条路径一并消失。
- Windows 安装器为了「MCP 宿主进程占着 exe」专门加了 CloseApplication 逻辑，删 MCP 后升级场景更简单。

**用户可见影响。** 设置里的「MCP」页消失；会话编辑框少一行「允许 MCP 访问此会话」（对话框高度变化，需同步改两个布局测试）。GUI 的连接、SFTP、终端、导入导出不受影响。

**耦合 / 风险。** 低。GUI 进程从不调用 `mcp::run`。真正的耦合只有三处：`automation` 的 `Frontend::Mcp` 分支、`ssh::connection` 中给自动化用的取消作用域（`with_automation_cancellation` / `inherit_automation_cancellation`，SFTP 也调用 `inherit_*`，保留 CLI 时不动；连 CLI 一起删时可以把它简化掉）、以及布局测试的像素偏移。

### 2. CLI + automation 层 + `headless` feature（建议删，需确认）

**它做什么。** `meatshell cli sessions|session|import|exec|files|read|upload|download`，复用 `automation` 层；`--features headless` 编出无 GUI 的服务器版本（目前只给远程 MCP 用）。

**涉及。** `src/cli/**` 309 行、`src/automation/**` 690 行、`src/ssh/impls/ssh.rs:1472-1600` 的 `execute_command(_inner)` + `CommandExecution` 约 130 行、`ssh/impls/connection.rs` 的自动化取消作用域及其测试约 80 行、`src/terminal/headless.rs`、`main.rs` 中 `cfg(feature = "headless")` 分支与 `[features] headless`、README「CLI」段、`tests/config_import_e2e.py`（142 行）、`tests/ssh_jump_chain_e2e.py`（478 行，用 `cli exec` 当跳板链测试驱动）。

**依赖。** 没有独占 crate（`fs2` 配置锁仍需保留：GUI 多窗口/多进程也会写配置）。GUI 包体积增量估计 <0.5 MiB（只有 automation 自身代码，未单独实测）。

**为什么说它也「没什么用」。**

- Windows release 是 `windows_subsystem = "windows"`，程序里没有 `AttachConsole`，所以在 cmd/PowerShell 里直接敲 `meatshell cli sessions` **看不到任何输出**（只有重定向到文件或管道时才看得到）。你主要用 Win11，等于这个功能在 Windows 上实际不可用。
- `cli exec/upload/download` 同样能用已保存凭据登录服务器，只是没有 MCP 的权限门（设计上认为是「本机用户主动操作」）。

**风险。** 失去两个 Python e2e 测试（它们不在 CI 里跑，靠手动）。跳板链逻辑仍有 Rust 单测覆盖 `config/jump_chain.rs`，GUI 连接走同一条 `ssh` 路径。

**建议。** 如果你不在命令行里用 `meatshell cli`，与 MCP 一起删，代码最干净（`automation`、`execute_command`、`headless` 全部可去）。如果想保留，只删 MCP，把 `Frontend` 枚举收缩为仅 CLI。

### 3. 杂物脚本 `tools/build_minimax_multishot.py`（删）

读取 `E:\ComfyUI\...` 的视频工作流并写 JSON，跟 SSH 客户端毫无关系，仓库内无引用（`SECURITY_AUDIT.md` L2 已记）。

### 4. AUR 打包（删）

`packaging/aur/PKGBUILD` 的 `url` 指向上游、`sha256sums` 为 `SKIP`；`aur-publish.yml` 的 `if:` 限定 `github.repository == 'yituorou/meatshell'`，在本 fork 永远不会运行。留着只有「有人在本树 `makepkg` 会下载上游包且不校验」的风险（M7）。

### 5. Flatpak（可选删）

`packaging/flatpak/**` 用上游 app-id `io.github.yituorou.meatshell`，release.yml 每次全平台发版都会跑一个 flatpak 容器 job。最近几版你只发 Windows。如果不打算在 Linux 上发 Flatpak，可以删掉，release 也能快一些；若想保留 Linux 发行，建议至少把 app-id 改成 fork 自己的。

## 三、其余功能逐项评估

| 功能 | 做什么 | 规模 / 依赖 | 安全相关 | 建议 |
|---|---|---|---|---|
| 启动更新检查 | 后台请求 `api.github.com/repos/woncc/meatshell/releases/latest`，有新版显示横幅，可在设置关闭（`update_check_disabled`） | ~40 行；`ureq` 被 WebDAV 共用，删了依赖数不变 | 只发 UA，不上传数据 | 保留 |
| WebDAV 同步 | 上传/下载口令加密的连接导出（#26） | `src/app/webdav.rs` 524 + `src/webdav/**` 62；`ureq`+`rustls`+`ring` 等 8 个 crate 仅它和更新检查用 | #26 后已改为口令加密 | 保留；「信任自签名证书」可选删或改为指纹固定（M2） |
| 单实例 IPC | `--new-window` 转发给已运行实例；Unix 用 `ipc.sock`，Windows 用 `127.0.0.1:随机端口` | 194 行，无依赖 | 只接受 `new-window` 一条指令，最坏是被本机进程弹出新窗口 | 保留 |
| Windows 跳转列表 | 任务栏右键「新建窗口」 | 118 行，`windows` crate 已有 | 无 | 保留 |
| 托盘 | 关窗后后台保持会话 | 153 行；`tray-icon` Windows −3 / 全平台 −6 crate | 无 | 保留 |
| 串口会话 | COM/ttyUSB | 260 行；`serialport` 全平台 −7（Linux 需 libudev） | 无 | 保留（运维常用） |
| Telnet | 明文 TCP | 294 行，无依赖 | 明文协议（L5） | 保留（设备调试仍需要），可考虑在 UI 标「不加密」 |
| ZMODEM（rz/sz） | 终端内传文件 | 916 行，无依赖 | 无 | 保留 |
| RDP 启动器 | 调 `mstsc` / `xfreerdp`，不实现协议 | 755 行，无依赖 | Windows 临时 `.rdp` 不删除、`authentication level:i:0` 关闭证书提示（M3） | 保留但修 M3；若你从不用 RDP 可「可选删」 |
| 彩色 Emoji（Twemoji PNG） | 终端中 emoji 显示为彩色图片（#269） | 依赖 −1，但内嵌 ~4.1 MB PNG（4009 张 72×72） | 无 | 保留（删掉 emoji 变单色；是体积大户，若追求体积可选删） |
| 系统深浅色检测 `dark-light` | 首次启动跟随系统主题 | Windows −1；Linux/macOS −20（zbus 等） | 无 | 保留 |
| 系统字体枚举 `fontdb` | 设置中选字体 | −2~4 | 无 | 保留 |
| 端口转发 `-L` / `-D` | 本地监听转发；`-D` 为无认证 SOCKS5 | `src/tunnel/**` 218 行 | 可绑定 `0.0.0.0`、默认自动启动（M4） | 保留；建议非 loopback 时二次确认 |
| 会话触发器（expect/自动回复） | 匹配输出自动发送文本 | ~60 行 + UI | 可能被用来自动发密码，属用户自配 | 保留 |
| JSON 美化输出 | 整行 JSON 自动格式化 | 244 行 | 无 | 保留 |
| 会话日志 | 按标签记录终端输出 | 450 行 | 日志含主机名，默认关 | 保留 |
| 进程窗口 / 系统信息 / 远程编辑器 | 辅助窗口 | UI ~1,300 行 | 远端 `ps` 每 2 秒（M8）；Windows 读 GPU 用 `-ExecutionPolicy Bypass`（L1） | 保留；建议修 L1、M8 |
| FinalShell 导入（`des`/`md5`） | 导入 FinalShell 连接 | 300 行；依赖数 0 变化 | 无 | 保留 |
| PuTTY PPK 加载 | `aes`/`cbc`/`hmac`/`argon2` | 551 行；都与 #26 共用 | 无 | 保留 |
| 内置壁纸 | `ms.jpg`（默认）、`miku.jpg` | 各 ~300 KB，各嵌入两次 | `miku` 为动漫角色图，有版权疑虑 | 删 `miku`，保留 `ms` |
| `--config-info` | 打印可执行路径、版本、数据目录、会话数 | ~15 行 | 无 | 可选删（无调用方） |
| `--data-dir` / `MEATSHELL_DATA_DIR` | 选择配置目录（便携/多配置） | config.rs ~30 行 | 无 | 保留（与 MCP 无关的通用功能；仅删 `--http-config` 专用检查） |
| 直接依赖 `thiserror`、`russh-keys` | 代码中无直接 `use` | 传递依赖仍在，数目 0 变化 | — | 从 Cargo.toml 删除（`tokio-util`、`url` 随 MCP 一起删） |
| 文档资产 | `assets/architecture-live.gif`（1.2 MB，画的是 GUI/CLI/MCP 架构）、`docs/QR/QQ_Group_QR_Code.jpg`（上游 QQ 群） | 仅仓库体积 | 无 | gif 删或重画；QQ 群码可选删 |

## 四、配置文件兼容性

- `ConfigFile` 与 `Session` 结构体**没有** `#[serde(deny_unknown_fields)]`（只有 MCP 自己的 `http.rs`/`oauth.rs` 有），所以删字段后，旧 `sessions.json` 里的 `mcp_enabled`、`mcp_use_saved_credentials`、`mcp_allow_commands`、`mcp_allow_file_transfers`、每个会话的 `mcp_access` 会被 serde **静默忽略**，读取不会报错；下次保存时这些键自然消失。
- 旧便携导出（含 `mcp_access` 的会话）导入同样不受影响。`import_tests.rs:160-190` 那条「导入不得覆盖本机 MCP 权限」的测试要删除或改写为「导入文件中多余的旧键被忽略」。
- 建议实施时补一个单测：用带全部旧 MCP 键的 JSON 反序列化 `ConfigFile`/`Session` 成功，并且重新序列化后不再出现这些键。
- 若同时删 RDP/壁纸等：`builtin:miku` 壁纸值需在加载时回落到默认壁纸（壁纸代码已有「找不到就回落」逻辑，实施时需确认）。
- 已安装 MCP 客户端的用户：客户端配置里的 `meatshell mcp serve` 会启动失败。建议保留 `mcp` 子命令的识别，只输出一行「此版本已移除 MCP」后以非零码退出，而不是落入 GUI 启动（否则 MCP 客户端会意外弹出一个 GUI 窗口）。CLI 若删除同理。

## 五、建议的实施顺序

1. **第一刀（低风险、收益最大）**：删 MCP 全套（代码、设置页、会话勾选、配置字段、依赖、文档、e2e、两个工作流、release.yml 中的文档拷贝、deb 资产、wix 文案）；删 `tools/build_minimax_multishot.py`、AUR；删 `thiserror`/`russh-keys` 直接依赖；`mcp` 子命令改为提示后退出。
2. **第二刀（需你拍板）**：CLI + automation + `headless` 一起删，或保留 CLI 并收缩 `Frontend`。
3. **第三刀（可选）**：Flatpak、`miku` 壁纸、WebDAV 自签证书开关、`--config-info`、README 架构 gif。
4. 每刀之后：`cargo +stable test`、Windows 编译、`cargo +stable tree` 复核依赖数，并在 Win11 上确认设置页与会话编辑框布局。

## 六、审查边界

- 依赖数基于 `Cargo.lock` 离线解析；二进制体积只实测了 MCP（Linux headless release）。GUI 包与 Windows 包的实际减少量需在实施后用真实发版产物复核。
- 没有运行 `cargo-udeps`（需要 nightly），未使用依赖靠逐 crate `rg` 引用判断。
- 本报告不修改任何代码，仅新增本文件。
