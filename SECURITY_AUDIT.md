# MeatShell 对抗性安全审计

| 项 | 值 |
| --- | --- |
| 仓库 | `woncc/meatshell` |
| 审计基线 | `work/main` @ `6459b7a`（v0.7.7） |
| 上游对照 | `87c9481`（`FORK_NOTES.md` 记载的分叉点） |
| 方法 | 只读静态审查：`src/`、`ui/`、`vendor/` 中的网络符号、`.github/workflows/`、`build.rs`、`scripts/`、`assets/install-linux.sh`、`packaging/`、`Cargo.toml` / `Cargo.lock`、以及 `87c9481..HEAD` 的 fork 差异 |
| 日期 | 2026-10-07 |

本报告找的是：隐蔽后门、间谍行为、凭据外传、主人可能没想到的出站，以及供应链风险。它**不是**渗透测试，也**没有**对发布产物做动态流量抓包。下面每一条都给出 `文件:行`。没有证据的地方会写成「未在审查范围内看到」，而不是「不存在风险」。

## 结论

在本次审查的应用源码、fork 差异和 CI 脚本里，**没有发现**会在用户不知情时把会话清单、密码或私钥发到固定第三方域名的代码路径。也没有发现 keylogger、远程截屏上传、远程脚本 `eval`、或用 base64 藏起来的 C2 载荷。

仍然有几类主人应当认真对待的风险，而且它们大多是**产品功能的默认值或文档里轻描淡写的设计**，不是藏起来的第二套逻辑：

1. 便携导出 / WebDAV 同步使用写死在源码里的密钥，拿到 JSON 就能解出密码和内联私钥；代理 URL 里的口令甚至不加密。
2. MCP 与 CLI 已从 `slim` 分支删除。旧的 `mcp` / `cli` 参数只打印一行说明并以非零状态退出，不再监听端口，也不能用已保存凭据执行命令。
3. 桌面版启动更新检查默认开启，请求的是**上游** `yituorou/meatshell`，不是本 fork。横幅不会自动安装，但会把用户带到上游发布页。
4. 「闭眼脱敏」只盖住界面上的字。真实主机、用户、端口仍在连接路径、配置文件、诊断日志和主机密钥对话框里。

## 严重级别

| 级别 | 含义 |
| --- | --- |
| Critical | 未授权、默认开启、且能把凭据或会话清单送到攻击者控制的目的地 |
| High | 默认可达，或拿到一份「已加密」文件就等于拿到密码；或一个明确的本机入口能用已存凭据操作全部会话 |
| Medium | 需要用户打开某个开关、点一次、或写错绑定地址，但后果是凭据落地、中间人或替换安装包 |
| Low | 范围有限、默认关闭，或只影响本机诊断 |
| Info | 合法产品行为，或审查边界 |

本次**没有**标 Critical 的发现。这只说明下面列出的路径不满足 Critical 的定义，不代表发布二进制或未逐行审计的传递依赖是安全的。

---

## High

### H1. 便携导出密钥写死在二进制里；WebDAV 上传整份会话清单

**发生了什么。** 本机 `sessions.json` 的密码、内联私钥、触发器应答和 WebDAV 口令，用 `secret.key` 里的随机 32 字节做 ChaCha20-Poly1305（`src/config/impls/config.rs:406-418`、`1764-1787`）。这是合理的本机加密。

导出和 WebDAV 走的是另一把钥匙。`EXPORT_KEY` 是源码里的 32 字节 ASCII：

```398:400:src/config/impls/config.rs
    /// Fixed 32-byte key for portable exports. Baked into the binary so an
    /// exported file decrypts on any machine. Obfuscation only — see `ExportFile`.
    const EXPORT_KEY: [u8; 32] = *b"meatshell.export.portable.key.01";
```

`export_json` 用这把钥匙重加密密码、内联私钥和触发器应答，主机 / 用户 / 端口 / 代理字符串保持明文（`src/config/impls/config.rs:1890-1916`）。注释自己写了这是混淆，不是保密（`src/config/struct/config_file.rs:344-350`）。

WebDAV **不会**在启动时自动同步（`src/app/webdav.rs:256-260`）。用户点上传后，`webdav_put_json` 对用户填写的 URL 发 HTTP `PUT`，正文就是这份导出 JSON（`src/app/webdav.rs:220-236`、`320-329`）。默认远端文件名是 `meatshell-connections.json`（`src/config/impls/config.rs:1561`）。

`proxy` 字段类型是普通 `String`，注释示例就是 `http://user:pass@host:8080`（`src/config/struct/session.rs:162-165`）。导出循环不处理 `proxy`，所以代理口令以明文出现在 `sessions.json` 和导出文件里。`private_key_path`、`note` 同样是明文。

**为什么要紧。** 任何能读到导出文件的人（NAS 备份、WebDAV 目录权限、误传到网盘）都可以用公开常量解密 `enc:exp:v1:` 载荷。这和「文件已加密」的直觉相反。

**建议。**

- 导出改为用户口令（Argon2/scrypt 派生）或只在目标机上已有的 `secret.key`。
- WebDAV 上传前明确提示：主机清单会离开本机，密码保护强度等于公开常量。
- 把 `proxy` 的 userinfo 拆成 `Secret`，至少与密码同一套加密。
- 不要把内联私钥放进可移植导出，除非用户单独确认。

### H2. MCP 预览默认全开（slim 分支已移除）

**状态。** 本分支已删除 MCP（stdio 与 HTTP/OAuth）、CLI、automation 层和 `headless` 构建。桌面进程不再提供这些入口，也不再因它们监听端口。`meatshell mcp ...` 与 `meatshell cli ...` 向 stderr 输出一行说明后以非零状态退出，不会打开窗口。

旧配置里的 `mcp_enabled`、`mcp_use_saved_credentials`、`mcp_allow_commands`、`mcp_allow_file_transfers` 和会话字段 `mcp_access` 仍能被读入（结构体没有 `deny_unknown_fields`），下次保存时不再写出。设置页和会话编辑框中的 MCP 控件已去掉。

删除前的风险是：这些开关默认全开，本机客户端执行 `meatshell mcp serve` 就能用已保存凭据在远端执行命令、传输文件。该路径在当前分支不存在。

---

## Medium

### M1. 启动更新检查默认打到上游仓库，而不是本 fork

**发生了什么。** 第一个窗口且 `update_check_enabled()` 时，后台线程：

```709:712:src/app/open_window.rs
                match ureq::get("https://api.github.com/repos/yituorou/meatshell/releases/latest")
                    .set("User-Agent", "meatshell-update-check")
                    .timeout(std::time::Duration::from_secs(8))
                    .call()
```

只解析 `tag_name`，不上传本机会话（`src/app/open_window.rs:717-732`）。失败被吞掉。`update_check_disabled` 默认 `false`，所以检查默认开启（`src/config/struct/config_file.rs:312-316`）。

横幅「下载」用系统浏览器打开 `https://github.com/yituorou/meatshell/releases/latest`（`src/app/misc_callbacks.rs:82-89`）。关于页打开 `https://github.com/yituorou/meatshell`（`src/app/misc_callbacks.rs:92-99`）。没有自动下载、没有校验和安装器。

更新检查的行为见本节，以及 `FORK_NOTES.md` 的更新渠道说明。对 fork 主人仍然是出站与供应链问题：每次启动的桌面进程都会向 `api.github.com` 暴露客户端 IP 和 User-Agent，并且 UI 引导用户安装**另一条发布线**的二进制，从而覆盖本 fork 的改动。

**建议。** 本 fork 构建把 URL 换成 `woncc/meatshell`，或默认 `update_check_disabled = true`，直到有自己的发布通道。不要只在文档里提醒。

### M2. WebDAV「接受无效证书」会关闭 TLS 校验

`webdav_accept_invalid_certs` 默认 false。打开后，`WebDavAcceptAnyCertVerifier` 对服务器证书和 TLS 1.2/1.3 签名一律返回成功（`src/webdav/impls/certificate_verifier.rs:5-37`，接入点 `src/app/webdav.rs:92-108`）。

这条路径和 H1 叠在一起：中间人可以读到整份导出 JSON，并用公开的 `EXPORT_KEY` 解密码。默认关闭，所以不是 Critical。

**建议。** 保持默认关闭；打开时在 UI 显示当前主机名，并拒绝把该开关和「上传全部连接」放在同一次无确认操作里。优先支持固定指纹（pin），而不是接受任意证书。

### M3. Windows RDP 把 DPAPI 口令写进临时 `.rdp`，且不删除；证书校验被关掉

Windows 不能把密码放在 `mstsc` 命令行。实现是把 DPAPI 密文写进 `%TEMP%/meatshell-rdp-{session_id}.rdp`（`src/rdp.rs:98-100`、`117-126`、`216-223`）。全仓库只有这一处引用 `meatshell-rdp`，没有删除。同一用户下的其他进程可以用 DPAPI 解回口令。文件里还有明文 `full address` 和 `username`。

另外写了 `authentication level:i:0`（`src/rdp.rs:142`），`mstsc` 因此不警告不受信任的 RDP 证书。注释说明这是为了自签证书。效果是 RDP 中间人更容易。

非 Windows 把密码从 stdin 交给 `xfreerdp` / `xfreerdp3`，不落盘（`src/rdp.rs:485-496`）。命令行参数测试要求密码不出现在 argv（`src/rdp.rs:397` 附近）。

**建议。** `mstsc` 启动后尽快删除 `.rdp`（并考虑用完即覆盖）。把证书策略做成显式开关，默认不要用 `authentication level:i:0`。

### M4. 端口转发可绑定非 loopback；动态转发是无认证 SOCKS5

本地 `-L` 和 `-D` 直接 `TcpListener::bind` 用户给出的地址，空地址才落到 `127.0.0.1`（`src/tunnel/impls/forward.rs:26-38`、`71-73`、`114-116`）。动态转发注释写明 SOCKS5 **无认证**、只做 CONNECT（`src/tunnel/impls/forward.rs:105-107`）。已保存规则默认 `auto_start: true`（`src/config/struct/session.rs:284-301`）。

用户若把绑定写成 `0.0.0.0`，这台机器会变成通往 SSH 服务器内网的开放代理。这是产品功能，但默认自动启动让一次错误配置在下次连接时生效。

**建议。** 非 loopback 绑定需要单独确认；`-D` 至少提供可选口令，或在绑定不是 loopback 时拒绝启动。

### M5. 「闭眼」是盖层，不是脱敏数据路径

`src/app/privacy.rs:1-6` 写明：保存的会话和真实连接保持原地址，只改窗口绘制的字符串。连接仍使用 `session.host` / `user` / `port`（`src/app/open_window.rs:660` 的注释也这么说）。

会话编辑框的 `fixed-mask` 是在输入框上盖一层 `****` 和一个 `TouchArea`，底层 `value` 仍双向绑定真实 `draft-host` / `draft-port` / `draft-user`（`ui/widgets.slint:204-218`，`ui/session_dialog.slint:898-909`）。辅助功能树、UI 自动化或能读 Slint 属性的工具仍看得到原文。

主机密钥对话框**故意不遮**。详情字符串是 `{host}:{port} ({key_type})` 加指纹（`src/app/auth_dialogs.rs:3-10`），原样送进 `hostkey-detail`（`src/app/auth_dialogs.rs:123`，`ui/app.slint:4906`）。这是正确的安全提示，但也说明闭眼不能用于「截图给别人看密钥对话框」。

诊断日志在 WARN 级写下主机名，例如 shell 退出（`src/ssh/impls/ssh.rs:2213-2224`）。文件层过滤器是 `warn`（`src/main.rs:168-176`），所以 `error.log` 会留下 host。会话日志头也包含 `user@host:port`（测试断言见 `src/terminal/impls/session_log.rs:438-444`）。全局会话日志默认关（`session_log_enabled` 的 `bool` 默认 false，`src/config/struct/config_file.rs:149`）。

剪贴板是按需读写（复制选区、粘贴、复制路径），没有后台监视。见 `src/app/helpers.rs:15-24`、`src/app/key_input.rs:843-874`。闭眼不会阻止用户把终端里出现的主机名复制出去。

**建议。** 在设置文案里写清：闭眼不改变连接、日志、主机密钥框和剪贴板。若目标是截图安全，编辑框应使用独立的显示字符串，而不是把真实值留在同一 `TextInput` 里。

### M6. `known_hosts` 与 `error.log` 没有强制 `0600`

`sessions.json` 临时文件在 Unix 上设为 `0o600`（`src/config/impls/config.rs:1792-1800`）。`secret.key` 同样（`src/config/impls/config.rs:492-496`）。`known_hosts` 的 `remember` 只 `write` 文件，不改权限（`src/ssh/impls/known_hosts.rs:88-110`）。`error.log` 用 `OpenOptions` 创建，也没有收紧模式（`src/logging/impls/error_log.rs:26`）。umask `022` 时，同机其他用户可以读到 `host:port` 与主机公钥，以及断开原因里的主机名。

主机名、用户名在 `sessions.json` 里本来就是明文（只有密码类字段加密）。这是 SSH 客户端的常态，但和「闭眼」容易被混为一谈。

**建议。** 创建 `known_hosts`、`error.log`、便携 `config/` 目录时使用 `0700`/`0600`。

### M7. CI 拉取未固定的构建工具

产品二进制里没有 `curl | sh`。发布工作流里有：

- Debian 10 兼容任务把 `https://sh.rustup.rs` 管道给 `sh`（`.github/workflows/release.yml:389-390` 与 `481-482`）。这是官方 rustup 安装方式，仍然是远程脚本执行，且没有单独的校验和。
- x86_64 AppImage 步骤 `wget` **continuous** 通道的 `linuxdeploy` 与插件，没有 sha256（`.github/workflows/release.yml:129-132`）。`continue-on-error: true`。一个被替换的 continuous 构建可以进入对外发布的 AppImage。该步骤只在打 `v*` 标签或手动跑 workflow 时执行。

原先用来对照的 MCP TLS 工作流、发布包校验工作流和 AUR 发布工作流已从本分支删除。`packaging/aur/` 一并删除，避免一份校验和为 `SKIP` 的配方在本树被 `makepkg` 误用。

**建议。** AppImage 工具改成带版本号和 sha256 的发布物。rustup 改为 actions 已有的 `dtolnay/rust-toolchain`（同文件其他 job 已经这么用），或固定 rustup 安装脚本哈希。AUR 配方已从本分支删除，不必再改 PKGBUILD。

### M8. 每次 SSH 连接都会在远端跑监控命令

这不是发回开发者的间谍逻辑，数据留在本机 UI。但范围比「打开进程窗口才采样」更宽。

只要没勾选 `disable_shell_integration`，连接后会自动开辅助通道（`src/ssh/impls/ssh.rs:1814-1870`）：

- `MON_CMD`：循环读 `/proc/stat`、`/proc/meminfo`、`/proc/net/dev`，以及 `df -kP`（`src/ssh/impls/ssh.rs:1783`）
- `PROC_CMD`：每 2 秒 `ps -eo pid,user,pcpu,pmem,rss,args`，取 CPU 前 40 行（`src/ssh/impls/ssh.rs:1802`）
- `SYS_CMD`：一次性 `hostname`、`hostname -I`、`uname`、`/etc/os-release`、`lspci` 等（`src/ssh/impls/ssh.rs:1787`）

PATH 被固定到系统目录，是为了避免远端 `PATH` 劫持（同函数上方注释）。命令是源码常量，不是从网络下载的脚本。

shell 集成还会在登录后注入一段 hook，用 `fc -ln -1` 把刚执行的命令经私有 OSC 697 送回客户端，写入本地命令历史（`src/ssh/impls/ssh.rs:1733-1760`，历史落在 `command_history`，`src/config/struct/config_file.rs:222`）。密码提示走 `read -s` 时不会进历史；若密码被当成普通命令执行，则会以明文进 `sessions.json`。

**建议。** 在会话里把「资源监控 / 进程采样 / 记录命令」拆开，进程 `ps` 默认不要在窗口未打开时跑。命令历史视为敏感，考虑加密或提供关闭 OSC 697 的开关（已有 `disable_shell_integration`，但粒度太粗）。

---

## Low

### L1. Windows 用 PowerShell `-ExecutionPolicy Bypass` 读本机 GPU

`fill_local_gpu_info` 在首次查询本机硬件时启动 `powershell -NoProfile -ExecutionPolicy Bypass -Command …`，命令是源码里的固定 WMI/注册表查询，不下载脚本（`src/app/resource_ui.rs:548-557`）。结果只进本机硬件信息结构。这不是远程代码执行，但 Bypass 会让安全产品告警。

**建议。** 改用 CIM/Win32 API，或去掉 `-ExecutionPolicy Bypass`（`-Command` 的本地字符串通常不需要它）。

### L2. `tools/build_minimax_multishot.py` 与产品无关（已删除）

该脚本与 SSH 客户端无关，已从本分支删除。

### L3. `Cargo.toml` 的 `repository` 仍是占位符

`repository = "https://github.com/your/meatshell"`（`Cargo.toml:8`）。crates.io 元数据或关于框若采用该字段，会指向不存在的上游。当前关于框用的是写死的 `yituorou` URL（见 M1），不是这个字段。

### L4. 仍链接有 SSH agent 帧解析问题的 russh 0.49

`audit.toml:1-23` 忽略 `RUSTSEC-2026-0154`，理由是本仓库不调用 ssh-agent。对 `src/` 搜索 `AuthAgent` / `ssh-agent` / agent forwarding，命中仅注释与 CHANGELOG。漏洞代码仍在依赖里。若未来有人打开 agent 转发，DoS 会变成可达。升级被推迟是因为修复版带 `-pre`/`-rc` 加密库，这个取舍在 `audit.toml` 里写清楚了。

**建议。** 保持忽略清单与一条测试：禁止新增 agent API 调用。russh 依赖离开 rc 后升级。

### L5. Telnet 是明文 TCP

`src/terminal/impls/telnet.rs` 对用户主机做 `TcpStream::connect`，没有 TLS。这是协议本身。口令会出现在路径上。

---

## Info — 合法产品行为（对照用）

这些会出网或读本机，但是用户发起的 SSH 客户端能力，审查中没有看到它们把数据副本送到开发者服务器。

| 行为 | 代码 | 目的地 |
| --- | --- | --- |
| SSH | `src/ssh/impls/ssh.rs` 中 `TcpStream::connect((session.host, session.port))`（约 1182、3145 行）及 russh | 用户会话主机，或跳板链 |
| Telnet | `src/terminal/impls/telnet.rs:129-133` | 用户主机 |
| RDP | `src/rdp.rs` 启动 `mstsc` 或 `xfreerdp3`/`xfreerdp` | 由系统客户端连接用户主机 |
| HTTP CONNECT / SOCKS5 | `src/ssh/impls/proxy.rs:81-114`；未设置会话代理时读 `ALL_PROXY`/`all_proxy`（`src/ssh/impls/proxy.rs:31-32`） | 用户代理 |
| 端口转发 | `src/tunnel/impls/forward.rs` | 本机监听 + SSH `direct-tcpip` 到用户目标 |
| WebDAV | `src/app/webdav.rs:220-253` | 仅手动上传/下载时的用户 URL |
| 更新检查 | `src/app/open_window.rs:709` | 见下方清单；默认开，指向**上游** |
| ICMP 延迟 | `src/resource/impls/latency.rs:322-335` | 当前 SSH/Telnet 主机。fork 新增。主机名经过字符白名单，拒绝 `-` 开头和 shell 元字符（`ping_host`，`src/resource/impls/latency.rs:81-98`） |
| 本机采样 | `sysinfo`，`src/resource/impls/system.rs` | 本机 CPU/内存/网卡/磁盘，不上送 |
| 本机进程以外的扫描 | 未发现超出侧栏、进程窗口、硬件信息和 `fontdb` 字体枚举的目录爬虫 | — |
| 单实例 | `src/app/single_instance.rs:98` `127.0.0.1:0` | 仅本机 |
| 剪贴板 | `arboard`，按快捷键 | 本机 |
| 会话日志 | `src/terminal/impls/session_log.rs` | 本机文件；注释说明不记录无回显按键。默认关 |
| 触发器 expect/send | `SessionTrigger`，`src/config/struct/session.rs:304-315` | 把应答写进**当前** SSH 通道。远端若伪造 expect 字符串，可能诱使客户端发出保存的应答。应答在 `sessions.json` 中加密 |

`build.rs` 只编译 Slint、在 MinGW 上探测 `libmcfgthread`、在 Windows 上嵌入图标。没有网络调用。

`assets/install-linux.sh` 用 `sudo install` 安装本地二进制和 `.desktop`，没有下载。

`vendor/` 下的 Rust 源码未检出 `ureq`、`reqwest`、`api.github`、`telemetry`、`sentry`。这是 Slint 的本地补丁（文本布局与 macOS DisplayLink），不是第二套网络栈。未把 `vendor/` 与 crates.io 原包做逐字节 diff。

应用依赖里**没有** `reqwest`。直接网络相关 crate：`ureq` 2（更新检查与 WebDAV）、`tokio`（TCP）、`tokio-socks`、`russh` 0.49。没有 WebSocket 客户端、没有崩溃上报 SDK。MCP 用过的 `axum` / `hyper` / `rmcp` 已从直接依赖移除。

### 本 fork 提交（`87c9481..6459b7a`）

`FORK_NOTES.md` 声称的范围与 `git diff --stat` 一致：侧栏图表、延迟 ping、进程列排序、终端粘贴与 scrollback、闭眼脱敏、文档。`Cargo.toml` 在该区间只有版本号一类的小改动，没有新增 HTTP 客户端。新增的出站只有：用户在本机侧栏切到「延迟」后，对**当前连接主机**执行系统 `ping`。没有新的硬编码域名。

闭眼功能的实现与文档一致：展示层替换，连接路径不改。不要把它当成流量或日志层面的隐私控制。

---

## 全部出站目的地

「出站」包括产品二进制里的客户端调用，以及只在 CI 里出现的下载。DNS 没有写死解析器，也没有 DoH；主机名都交给操作系统解析器。

### 产品二进制（桌面）

| 目的地 | 协议 | 用途 | 何时 | 代码 | 分类 |
| --- | --- | --- | --- | --- | --- |
| `api.github.com`（路径 `/repos/yituorou/meatshell/releases/latest`） | HTTPS GET | 比较 `tag_name` 与 `CARGO_PKG_VERSION`。User-Agent `meatshell-update-check`。无请求体 | 第一个窗口，且未关闭更新检查。默认开 | `src/app/open_window.rs:709` | **意外（对本 fork）**。对上游仓库是声明过的功能 |
| `github.com`（`/yituorou/meatshell/releases/latest`） | 交给 `explorer` / `open` / `xdg-open` | 更新横幅 | 用户点击 | `src/app/misc_callbacks.rs:83` | **意外（对本 fork）** |
| `github.com`（`/yituorou/meatshell`） | 同上 | 关于页 | 用户点击 | `src/app/misc_callbacks.rs:93` | **意外（对本 fork）** |
| 用户会话的 host:port | TCP，SSH | 交互 shell、SFTP、监控通道 | 用户连接 | `src/ssh/impls/ssh.rs` | 合法 |
| 跳板会话的 host:port | TCP，SSH | `ProxyJump` 式多跳 | 会话配置了跳板 | `src/config/jump_chain.rs`、`src/ssh/impls/ssh.rs` | 合法 |
| 用户 Telnet host:port | TCP | 明文 Telnet | 用户连接 | `src/terminal/impls/telnet.rs:129` | 合法（明文） |
| 用户 RDP 主机 | 由 `mstsc` 或 FreeRDP 发起 | 远程桌面 | 用户打开 RDP 会话 | `src/rdp.rs` | 合法 |
| 会话代理或 `ALL_PROXY` | TCP，SOCKS5 或 HTTP CONNECT | 隧道 SSH/Telnet | 配置了代理或环境变量 | `src/ssh/impls/proxy.rs:24-32`、`104-114` | 合法 |
| WebDAV URL（用户填写，无默认主机） | HTTP 或 HTTPS，`PUT`/`GET`/`MKCOL` | 上传或下载 `meatshell-connections.json` | 仅按钮。默认不启用 | `src/app/webdav.rs:185-253` | 合法，但载荷见 H1 |
| 当前 SSH/Telnet 主机 | ICMP（系统 `ping`，一次，超时约 2 秒） | 侧栏延迟 | 本机面板切到延迟后 | `src/resource/impls/latency.rs:106-167`、`322-335` | 合法。fork 新增。不发到第三方 |
| 端口转发目标 | 经 SSH `direct-tcpip`，或远端 `-R` | 隧道 | 规则 `auto_start` 或用户启动 | `src/tunnel/impls/forward.rs` | 合法 |
| `127.0.0.1` 临时端口 | TCP | 单实例唤醒 | 启动第二进程时 | `src/app/single_instance.rs:98` | 合法，仅本机 |

未发现的出站类型：WebSocket 客户端、崩溃上报、统计、CDN 拉取壁纸（壁纸是内置绘制或本地文件，`src/wallpaper/impls/wallpaper.rs:31-41`）、硬编码的公网 IP（`src/` 里的 IP 是测试夹具或 `127.0.0.1`）、对 OAuth issuer 的运行时 HTTP 请求（JWKS 是本地文件）。

### 仅 CI / 打包脚本（不进 `meatshell` 进程）

| 目的地 | 用途 | 代码 | 分类 |
| --- | --- | --- | --- |
| `https://sh.rustup.rs` | Debian 10 容器里安装 Rust | `.github/workflows/release.yml:389`、`481` | 构建期。远程脚本，见 M7 |
| `github.com/linuxdeploy/linuxdeploy` 的 `continuous` AppImage，以及 `linuxdeploy-plugin-appimage` 的 `continuous` | 打 AppImage | `.github/workflows/release.yml:129-132` | 构建期。未固定哈希，见 M7 |
| `archive.debian.org` / `deb.debian.org` | Debian 10 容器 apt | `.github/workflows/release.yml:367-370` | 构建期 |
| `pub.freerdp.com/releases/freerdp-3.21.0.tar.xz` | Flatpak 构建 FreeRDP | `packaging/flatpak/io.github.yituorou.meatshell.yml:58` | 打包清单，不是运行时 |

`github.com` 上的 `actions/checkout`、`dtolnay/rust-toolchain`、`Swatinem/rust-cache`、`softprops/action-gh-release` 是 Actions 市场步骤，只在 GitHub 托管 runner 上执行。

---

## 凭据落点（本机）

| 数据 | 位置 | 保护 |
| --- | --- | --- |
| 密码、内联私钥、触发器应答、WebDAV 口令 | `sessions.json` | 本机 `secret.key` + ChaCha20-Poly1305，`enc:v1:`。Unix `0600` |
| 本机密钥 | 同目录 `secret.key` | 随机 32 字节，Unix `0600`。与 `sessions.json` 放在一起；拿到目录等于拿到明文 |
| 主机、端口、用户、代理 URL、私钥路径、备注、命令历史 | 同文件明文 | 无 |
| 导出 / WebDAV | 用户选择的路径或用户 URL | `enc:exp:v1:` + 公开 `EXPORT_KEY`。代理 userinfo 仍明文 |
| 主机公钥 | `known_hosts` | TOFU。权限未收紧。未知与变更都会弹窗，没有「一律接受」的当前路径（`src/ssh/impls/known_hosts.rs:65-84`，校验在 `src/ssh/impls/ssh.rs:2990` 一带） |
| 诊断 | `log/error.log`，上限 50 MiB 后截断重写（`src/main.rs:168-176`） | WARN+。含主机名。按键只记长度（`src/ssh/impls/ssh.rs:1897-1899`）。进程控制日志会把口令替换成 `[REDACTED]`（`src/ssh/impls/ssh.rs:841-847`） |
| 终端记录 | 用户设置的目录 | 默认关。记录的是屏幕文本，不是原始按键（`src/terminal/impls/session_log.rs:1-7`） |
| RDP | `%TEMP%/meatshell-rdp-*.rdp` | DPAPI，但是不删除。见 M3 |
| 数据目录 | 可执行文件旁的 `config/`，不可写时才是 `%APPDATA%/meatshell` 或 `~/.config/meatshell`（`src/config/impls/config.rs:46-62`、`104-113`） | 便携目录若在共享盘上，`0600` 仍挡不住同一用户的其他程序 |

`Secret` 的 `Debug` 红acted，`Drop` 时 `zeroize`（`src/config/struct/secret.rs:36-49`）。这能减少日志和释放后堆上的残留，不能防止导出密钥或明文 `proxy`。

未发现把会话清单上传到除用户 WebDAV 以外的地方。更新检查的 GET 不带会话字段。

---

## 依赖面（值得点名的直接依赖）

来自 `Cargo.toml`，不是传递依赖的完整清单。

| Crate | 能力 | 在本仓库中的用途 |
| --- | --- | --- |
| `russh` 0.49 / `russh-sftp` / `ssh-key` | SSH | 核心。见 L4。直接依赖 `russh-keys` 已移除，传递依赖仍可能存在 |
| `tokio`（含 `net`） | TCP | SSH、Telnet、转发、单实例 |
| `ureq` 2 + rustls | HTTPS 客户端 | **仅**更新检查与 WebDAV |
| `tokio-socks` | SOCKS5 客户端 | 用户代理 |
| `sysinfo` 0.33 | 本机进程与资源 | 侧栏与硬件信息。不是隐蔽扫描器 |
| `arboard` | 剪贴板 | 用户复制粘贴 |
| `serialport` | 串口 | 串口会话 |
| `portable-pty` | 本地 PTY | 本地 shell |
| `chacha20poly1305` / `zeroize` / `aes` / `argon2` / `des` / `md5` | 加密 | 本机秘密、PPK、FinalShell 导入。`des`+`md5` 只用于解密 FinalShell 的导出（`src/config/impls/finalshell.rs`），不用于新的存储 |
| `base64` | 编码 | 密文、PPK、代理 Basic 认证。没有第二层解码后执行 |

`Cargo.lock` 中有 `ureq`。没有名为 `reqwest` 的包。

---

## 建议的处理顺序

1. 本 fork 的更新 URL 改到 `woncc/meatshell`，或默认关掉启动检查（M1）。这是本仓库特有的出站。
2. 停止用公开 `EXPORT_KEY` 保护会离开本机的密码；代理口令不要明文进 JSON（H1）。
3. MCP / CLI 已从 `slim` 分支删除，不再通过默认开关暴露已保存凭据（原 H2）。
4. 删除或覆盖 Windows 临时 `.rdp`；收紧 `known_hosts` / `error.log` 权限（M3、M6）。
5. 固定 CI 里的 linuxdeploy 与 rustup（M7）。
6. `tools/build_minimax_multishot.py` 已删除（L2）。

## 审查边界

做了：应用与 UI 源码的 URL / 套接字 / 进程启动 / 秘密存储搜索；fork 差异文件列表；工作流与 `build.rs`；`vendor/` 的网络关键字；`Cargo.toml` 直接依赖。

没做：对 `cargo build` 产物做流量抓包；反编译每一个传递 crate；把 `vendor/i-slint-*` 与 crates.io 1.16.1 做完整 diff；运行时确认 GitHub 更新检查的 TLS 证书钉扎（代码使用 ureq 默认 WebPKI，没有自定义钉扎）。传递依赖或被替换的 CI 工具仍可能在**构建机**上出站，这不体现在 `meatshell` 的源码调用图里。
