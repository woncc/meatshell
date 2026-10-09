# woncc/meatshell 补充说明

本文档说明本仓库（[`woncc/meatshell`](https://github.com/woncc/meatshell)）相对上游 [`yituorou/meatshell`](https://github.com/yituorou/meatshell) 在本轮迭代中**实际落地**的改动与使用特色。内容仅依据本 fork 的提交、Issue、PR 与发版记录整理，不夸大未经验证的能力。

| 项 | 说明 |
| --- | --- |
| 上游 | [yituorou/meatshell](https://github.com/yituorou/meatshell) |
| 本仓 | [woncc/meatshell](https://github.com/woncc/meatshell) |
| 日常开发分支 | `work/main` |
| 基线（分叉点） | `87c9481`（与当时 `upstream/main` / `origin/main` 对齐） |
| 当前发版 | [v0.7.7](https://github.com/woncc/meatshell/releases/tag/v0.7.7)（`12bb52f`） |
| 对照窗口 | 自 `87c9481` 起至 v0.7.7，共约 14 个本 fork 独有提交 |

---

## 简介

MeatShell 本身是轻量 SSH / 终端客户端（Rust + Slint）。本 fork 在保留上游能力的前提下，围绕 **Win11 日常使用反馈** 做了一轮界面与交互修正：资源侧栏图表更可读、进程窗口更好用、终端粘贴与全屏程序历史更稳、以及便于截图的身份脱敏开关。

本仓继续从上游只读同步；工作成果推送到本 fork 的 `work/main`，并在本仓打 `v*` 标签发版。

---

## 相对原版的升级点

下列条目均有对应提交与 Issue / PR，可在本仓追溯。

### 终端交互

| 能力 | 说明 | 依据 |
| --- | --- | --- |
| 右 Shift+Insert 粘贴 | Windows 上右 Shift 常被当成 `KeyLocation::Standard`，导致 Shift 修饰丢失、无法粘贴；现与左 Shift+Insert 行为一致。Ctrl+V / Ctrl+Shift+V 不变。 | #8 · `850f1b8` |
| 交替屏退出后保留滚动历史 | 运行 `top` 等全屏程序时，ncurses `rmcup` 发出的 CSI 3 J 不再误清主屏 scrollback；主屏上的 CSI 3 J 仍可正常清屏。 | #14 · `81592ed` |

### 资源侧栏（速率 / 延迟）

| 能力 | 说明 | 依据 |
| --- | --- | --- |
| 拉宽显示更长时间 | 柱宽固定约 3px；加宽侧栏展示更多历史采样，而不是把柱子拉粗。缓冲按可拖到的最宽侧栏预留。 | #9 · `b46acb2` · PR #16 |
| 本机面板可切「延迟」 | 下方面板默认仍为本机实时上下行；可在设置或面板标题切到对当前 SSH/Telnet 主机的 ICMP 往返延迟（ms）。 | #10 · `3963165` |
| 上下行配色 + Y 轴 | 上行 / 下行默认绿 / 蓝可区分，设置里可分别改色；共享一条速率刻度（峰值 / 中点 / 0）。 | #11 · `78e5912` |
| 宽侧栏铺满 + 紧凑延迟图 | 最大宽度下折线图铺满；延迟模式去掉上方空白带；速率 Y 轴用短标签（如 `987.6M`、`1.234G`）。 | #15 相关跟进 · `a2dc503` · PR #16 |
| 按可见窗口重算 Y 轴 | 缩窄侧栏后，Y 轴只按**当前画出来的样本**定标，不再被已滚出可视区的旧尖峰抬高；拉宽后再纳入可见样本。适用于服务器速率、本机速度、延迟三图。 | #15 · `25f4505` · PR #16 |

### 进程窗口

| 能力 | 说明 | 依据 |
| --- | --- | --- |
| 列排序 + 实际内存 | 表头可点升/降序并保留当前列标记；增加按 RSS 格式化的内存实际大小列（与应用内其它单位风格一致）。 | #12 · `2680548` |

### 隐私与截图

| 能力 | 说明 | 依据 |
| --- | --- | --- |
| 睁眼 / 闭眼脱敏 | 工具栏眼睛与设置中的开关可将用户名、主机等显示为 `****`；默认明文。已保存会话与真实连接地址不变。 | #13 · `dfa71f6` |
| 端口一并脱敏 | 闭眼时用户名、主机、端口折叠为固定 `****`（会话列表、状态栏、标签、侧栏、编辑框、跳板列表、登录提示等）；主机密钥信任对话框仍可读。 | #13 跟进 · `dac1712` · PR #16 |

---

## 界面与交互特色（本 fork）

相对上游默认体验，本 fork 在 Win11 场景下更强调：

1. **侧栏像「时间轴」而不是「粗细条」** — 拖宽看更久历史，柱距保持细密。
2. **刻度跟眼睛走** — Y 轴跟当前可见样本走，缩窄立刻重算，避免「虚高刻度、柱子贴底」。
3. **本机面板可选延迟** — 需要链路质量时切到 ms，不必再开其它工具。
4. **截图友好** — 一键闭眼，连接信息在界面上收成 `****`，连端口也不漏。
5. **进程表可排可读** — 列排序 + 真实内存大小，排查占用更直接。
6. **全屏命令不再「吃掉」历史** — `top` 退出后仍能回滚之前的输出。

> 说明：#10 / #13 / #15 代码已合入 v0.7.7，Issue 仍保持打开，等待 Win11 实机确认后再关。

---

## 隐私与安全

- **脱敏只改展示**：闭眼模式替换 UI 文案；SSH 真实用户名、主机、端口仍用于连接。
- **默认明文**：未打开眼睛开关时行为与平常一致。
- **主机密钥对话框保持可读**：避免误信主机时看不清关键信息。
- **更新渠道**：应用内启动更新检查和「下载」横幅指向本 fork 的 [Releases](https://github.com/woncc/meatshell/releases)。横幅只打开网页，不会自动安装。

本轮未声称新增加密、审计日志或其它安全子系统；上表以外的安全能力仍以上游为准。

---

## 性能与稳定性

本轮 fork 提交以 **功能修正与交互** 为主，没有单独的「大幅降内存 / 加速」类变更可写进亮点。

与稳定性相关、且有实据的两点：

- 交替屏 / `top` 退出后主屏滚动历史不再被误清（#14）。
- 右 Shift+Insert 在 Win11 上可粘贴（#8），减少输入路径上的踩坑。

单元 / 集成测试覆盖了粘贴、scrollback、侧栏历史、速率轴、延迟、privacy 等相关用例（见各 Issue 评论与 PR #16 的 test plan）；**完整 Win11 窗口拖拽与实机点击仍以用户反馈为准**。

---

## 版本与下载

请从 **本 fork** 的 Releases 下载，不要从上游 Releases 覆盖安装（否则会丢掉本说明中的改动）。

- 最新版：[v0.7.7](https://github.com/woncc/meatshell/releases/tag/v0.7.7)
- 全部版本：<https://github.com/woncc/meatshell/releases>
- 开发线：`work/main`（发版前合并、打 `v*` 标签触发 Actions）

v0.7.7 已提供 Windows（`.msi` / `.zip`）、Linux（tar / AppImage / deb / flatpak 等）与 macOS（`.zip`）产物。

近期本 fork 发版节奏：

| 版本 | 大致内容 |
| --- | --- |
| [v0.7.6](https://github.com/woncc/meatshell/releases/tag/v0.7.6) | #8–#14 首轮修正合入（粘贴、scrollback、进程排序、侧栏时间窗、延迟切换、速率配色与 Y 轴、眼睛脱敏） |
| [v0.7.7](https://github.com/woncc/meatshell/releases/tag/v0.7.7) | PR #16：宽侧栏铺满、可见窗口定标、紧凑延迟图、短 Y 轴标签、端口脱敏 |

---

## 与上游的关系

| 策略 | 说明 |
| --- | --- |
| 性质 | 公开 fork，保留上游协议与大部分产品能力 |
| 推送 | 仅向本仓（`origin`）推送，日常分支为 `work/main` |
| 上游 | `upstream` 只读拉取；**不向** `yituorou/meatshell` 开 PR（本 fork 策略） |
| 发版 | 在本仓打 `v*` 标签，由本仓 GitHub Actions 构建多平台包 |
| Force-push | 禁止对已发布分支做 force-push |

分叉以来的独有提交可用：

```bash
git fetch upstream
git log --oneline upstream/main..work/main
```

（在当前工作树上，上述区间即 `87c9481` 之后至 v0.7.7 的本 fork 改动。）

---

## 相关 Issue / PR 索引

| 编号 | 标题摘要 | 状态（截至 v0.7.7 文档起草时） |
| --- | --- | --- |
| [#8](https://github.com/woncc/meatshell/issues/8) | 右 Shift+Insert 粘贴 | 已关闭 |
| [#9](https://github.com/woncc/meatshell/issues/9) | 侧栏加宽应显示更长时间 | 已关闭 |
| [#10](https://github.com/woncc/meatshell/issues/10) | 本机面板速度 / 延迟切换 | 开放（待 Win11 反馈） |
| [#11](https://github.com/woncc/meatshell/issues/11) | 上下行颜色 + Y 轴 | 已关闭 |
| [#12](https://github.com/woncc/meatshell/issues/12) | 进程列排序 + 实际内存 | 已关闭 |
| [#13](https://github.com/woncc/meatshell/issues/13) | 睁眼/闭眼脱敏 | 开放（待 Win11 反馈） |
| [#14](https://github.com/woncc/meatshell/issues/14) | top 后终端历史 | 已关闭 |
| [#15](https://github.com/woncc/meatshell/issues/15) | 可见窗口重算 Y 轴 | 开放（待 Win11 反馈） |
| [#16](https://github.com/woncc/meatshell/pull/16) | 宽图铺满 / 可见定标 / 端口脱敏 | 已合并 |

---

## 维护说明

- 本文档随本 fork 演进更新；新一轮 Win11 验收或合入上游同步后，应增补版本表与索引。
- 产品总览、构建与通用用法仍以 [`README.md`](./README.md) 与上游文档为准；**本页只写 fork 差异**。
- 欢迎在本仓 Issue 反馈；确认无误后再关闭仍开放的 #10 / #13 / #15。
