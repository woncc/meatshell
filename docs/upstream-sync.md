# 上游同步记录

本文分开记录两件事：

1. **审查到哪**。分类清单终点不是逐行审完，也不是已经合入。
2. **哪些已合入**。只有表里写了 fork 提交的行才算吸收了上游改动。

没合入不等于有缺陷。远端提交数量也不是未修缺陷的数量。

这不是版本发布说明，也不改版本号。发布说明若要引用，只用下面的「类别摘要」。

上游只读：[yituorou/meatshell](https://github.com/yituorou/meatshell)。本记录写在 [woncc/meatshell](https://github.com/woncc/meatshell)。

## 类别摘要

五个类别只描述这 98 个非合并提交。合并提交另计，见文末。

- 已吸收：7。两侧都有对应提交，哈希不同。
- 缺陷属实待修：10。当前代码上对过，还没有议题。
- 已审明确拒绝：19。有议题记录、版本边界或已读过的补丁范围。
- 新功能/分支不适用：46。不纳入，也不是缺陷。其中多数只看了提交说明，没有逐行读 diff。
- 未审：16。还不能下缺陷或拒绝结论。
- main 与 slim 类别不同：1（2713e5d）。

## 水位

| 名称 | 提交 | 含义 |
| --- | --- | --- |
| 同步基线 | `87c94815cc53537cc5d0ab57815eb03a9d297504` | `origin/main`、`origin/slim` 与 `upstream/main` 的 merge-base。2026-10-07，`refactor(app): split app.rs into focused modules`。 |
| 分类清单终点 | `6b28b0e00c12c936c0c1e87e939194e426a6d5dc` | 2026-11-09 21:57 +0800，上游 `Merge pull request #552`。基线之后到这里有 127 个提交，其中 98 个非合并提交已列入下表。 |
| 完整逐行审查水位 | 不前移，仍停在同步基线 | 「未审」和只依据提交说明的「新功能/分支不适用」没有逐行读 diff。不能把 `6b28b0e` 写成已完整审查。 |
| 本次写入时的上游头 | `6b28b0e00c12c936c0c1e87e939194e426a6d5dc` | 再拉取后没有更新。`6b28b0e..upstream/main` 为空。 |

以后上游头如果超过 `6b28b0e00c12c936c0c1e87e939194e426a6d5dc`，新范围只标未审，不自动把清单终点或逐行水位往前移。

## 数量不是缺陷数

127 个上游提交里，98 个是非合并提交，其余是合并提交。98 里面只有「缺陷属实待修」对应当前 fork 上核对过的问题。已吸收的已经在分支里。已审明确拒绝的是有原因不拿。新功能不适用的本来就不是 bug。未审的还没有结论。

## 分支头（本次写文档时）

| 分支 | 头提交 | 说明 |
| --- | --- | --- |
| `main` | `3680802085acd19cf1ef424b163a0bced45e10f6` | 2026-11-10 11:43:46 +0800 合并 [PR 29](https://github.com/woncc/meatshell/pull/29)。已审 head `e12e1b8cd0cabdb3edd2430494405893cf536494`。只把 rz 文件框挂到当前窗口，不是上游同步。 |
| `slim` | `55b6b0c7232a759a5fab939eab363361cdfca699` | 2026-11-10 11:43:51 +0800 合并 [PR 30](https://github.com/woncc/meatshell/pull/30)。已审 head `e90738720fb45c7fe55fc7c1fa6ae0e7a09a7ec6`。同样只是 rz 文件框。 |
| 两分支共同祖先 | `2549ea3781b72bb975ed2b8b42bd8ca37d689358` | fork 自己的 CI 提交，晚于同步基线。 |

`slim` 从这个祖先去掉了 MCP、CLI、automation、headless 和 AUR。不要把 `main` 整支并进 `slim`，也不要加回这些能力。

## 已合入，但不是上游整包

议题已按完成关闭。关闭没有再做一次上游合并。实机点选没有做，见文末，不把已经在分支上的代码说成没完成。

| 项 | main | slim | 议题 | 改动请求 |
| --- | --- | --- | --- | --- |
| 导出口令（`MSHX`，Argon2id） | 已包含 [`79d573e`](https://github.com/woncc/meatshell/commit/79d573eceb106c55b993d090d643e433d1d02df5) | 同一提交是 slim 的祖先 | [议题 26](https://github.com/woncc/meatshell/issues/26) 已关闭，完成 | 无关联改动请求 |
| 去掉 MCP、CLI、automation、headless、AUR | 保留 | 已包含 [`a83330b`](https://github.com/woncc/meatshell/commit/a83330b01aed40bf28e01d82cadd9c4d757f9224) 与 [`9cc6e71`](https://github.com/woncc/meatshell/commit/9cc6e71973d487fe16a49428453140ba9f5cd7e8) | [议题 27](https://github.com/woncc/meatshell/issues/27) 已关闭，完成 | 无关联改动请求 |
| 下表 7 个已吸收上游提交 | 有 | 有，哈希不同 | [议题 28](https://github.com/woncc/meatshell/issues/28) 已关闭，完成 | 无关联改动请求。格式化提交 [`52b4e67`](https://github.com/woncc/meatshell/commit/52b4e67880a6a8a28ac4fddbb11ddf381a7bca80)、[`6a977dc`](https://github.com/woncc/meatshell/commit/6a977dca6c7d2d39dab43ad34a2fce4f41f734b3) 不是上游 SHA |
| rz 文件框挂到当前窗口 | [PR 29](https://github.com/woncc/meatshell/pull/29) | [PR 30](https://github.com/woncc/meatshell/pull/30) | 关联已关闭的议题 28 | 见左 |

议题 26、27、28 的 `closedAt` 都是 2026-11-10T03:50:09Z，`stateReason` 为 `COMPLETED`。写本文时未结议题列表为空。

## 98 个非合并提交

| 上游 | 日期 | 题目 | main | slim |
| --- | --- | --- | --- | --- |
| [`4302e14`](https://github.com/yituorou/meatshell/commit/4302e14f3d5b49e8898c1debb80f111147bfa1a0) | 2026-10-07 | chore: 清理编译警告中的死代码与无用导入 | 已吸收 [`4c535d6`](https://github.com/woncc/meatshell/commit/4c535d69b184d098311d9145174a8d8618ce46e9)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`6328d25`](https://github.com/woncc/meatshell/commit/6328d25735128e178fb5529b1b8efa30efe147e2)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`806c120`](https://github.com/yituorou/meatshell/commit/806c12094718625fef98fc5f7ed70a9468f7a931) | 2026-10-08 | 退出ssh session时不清除历史，关闭tab时才清除历史 | 已审明确拒绝。#28 写明不合并：断开后保留历史会在后台标签清历史、备用屏不重置解析器、光标行越界，并退回断开释放内存。本次未逐行重读 diff。 | 同 main |
| [`1605cf6`](https://github.com/yituorou/meatshell/commit/1605cf62625de504f75ead52a81fc8df7cadf100) | 2026-10-08 | 重新启动ssh session时不清除历史 | 已审明确拒绝。#28 写明不合并：断开后保留历史会在后台标签清历史、备用屏不重置解析器、光标行越界，并退回断开释放内存。本次未逐行重读 diff。 | 同 main |
| [`2ea7d10`](https://github.com/yituorou/meatshell/commit/2ea7d108f4ecfeff93cfe2bce3ddeb8f62516004) | 2026-10-08 | 调整cursor_row和cursor_col以适应新的代码 | 已审明确拒绝。#28 写明不合并：断开后保留历史会在后台标签清历史、备用屏不重置解析器、光标行越界，并退回断开释放内存。本次未逐行重读 diff。 | 同 main |
| [`d31e619`](https://github.com/yituorou/meatshell/commit/d31e619141e533c38520a6b62154e813f2666fdf) | 2026-10-08 | 调整显示历史的行为：当cursor_row >= rows时，调整 view_offset = 0 | 已审明确拒绝。#28 写明不合并：断开后保留历史会在后台标签清历史、备用屏不重置解析器、光标行越界，并退回断开释放内存。本次未逐行重读 diff。 | 同 main |
| [`8fc6870`](https://github.com/yituorou/meatshell/commit/8fc68705c47a1a4137b9b30333ee0459ca6d1d27) | 2026-10-08 | feat: choose existing serial ports from SerialPortCombo | 已吸收 [`d37c295`](https://github.com/woncc/meatshell/commit/d37c295c73adc6b245c2d16b37877fb3db56cf28)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`fffee48`](https://github.com/woncc/meatshell/commit/fffee48be4c20d95810cd50ae817c45005aadde6)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`fd83068`](https://github.com/yituorou/meatshell/commit/fd8306836fe23c609fbd349056b338ba30890906) | 2026-10-09 | fix(macos): disable the system tray to stop the launch crash (#486) | 已吸收 [`dd9754b`](https://github.com/woncc/meatshell/commit/dd9754bea5df2d58ab3a506673ad36dd0829f257)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`45e5a5f`](https://github.com/woncc/meatshell/commit/45e5a5f676333b49679c211785f35bdadf784469)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`69500e5`](https://github.com/yituorou/meatshell/commit/69500e5d4ba5088352081e98aa9f5ba9e78b2e70) | 2026-10-09 | fix(macos): stop Touch Bar crash when a window closes (#488) | 已吸收 [`586c849`](https://github.com/woncc/meatshell/commit/586c849789ab16d0e0c575edd0168b51ba70b638)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`392f816`](https://github.com/woncc/meatshell/commit/392f816baadf45412c72f8ec1567413666434ee9)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`80db320`](https://github.com/yituorou/meatshell/commit/80db3203a15a0eff1631e5997f4d8b650be39af5) | 2026-10-10 | fix(terminal): upgrade vt100 to 0.16.2 to stop crash after resizing in nano/vim (#490) | 已吸收 [`0f1b97a`](https://github.com/woncc/meatshell/commit/0f1b97a54b03a494d6d442e58f066b7473872528)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`2011981`](https://github.com/woncc/meatshell/commit/20119811cc225c985b0f09ae3dffe511e8f880b3)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`0527db7`](https://github.com/yituorou/meatshell/commit/0527db7f8a0937b247834d79e44aa93f82118d11) | 2026-10-10 | feat(ui): add always-on-top pin for windows (#450) | 已吸收 [`7df6e58`](https://github.com/woncc/meatshell/commit/7df6e58022a6c1dc884c205988a136ab96bfbc67)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`f95099e`](https://github.com/woncc/meatshell/commit/f95099e220790c439f00cde7f00b653e48d0dd64)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`68d48f2`](https://github.com/yituorou/meatshell/commit/68d48f21f86ebedf75024895c027b438991102ca) | 2026-10-10 | feat(ui): open command history at the newest entry (#447) | 已吸收 [`73eb5db`](https://github.com/woncc/meatshell/commit/73eb5db70635a9629f833e34602595cd6e8b7f4c)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 | 已吸收 [`f9c46f1`](https://github.com/woncc/meatshell/commit/f9c46f1707e6ad67a78623b945c3bdc14d20a61e)。[议题 28](https://github.com/woncc/meatshell/issues/28)。cherry-pick -x 的 trailer 与本行上游 SHA 一致。 |
| [`f7752b3`](https://github.com/yituorou/meatshell/commit/f7752b3ac265e36700be8e022b8a7253225b3d42) | 2026-10-10 | chore(release): 0.7.6 | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`f654b61`](https://github.com/yituorou/meatshell/commit/f654b6149d143acc2e57d03909ada023cd9480f2) | 2026-10-10 | 正确的关闭tab时才清除历史（修复Bug：后台标签或侧栏隐藏时断开照样清历史） | 已审明确拒绝。只修补 #28 已拒绝的保留历史。fork 没有那条功能，合入会改错断开路径。本次未逐行重读 diff。 | 同 main |
| [`38c1e1b`](https://github.com/yituorou/meatshell/commit/38c1e1b6b8a48fbdeeaf90346798bce179327493) | 2026-10-10 | 修复：在 vim 等备用屏幕里断线重连后解析器不重置，新会话输出落到旧画面上 | 已审明确拒绝。只修补 #28 已拒绝的保留历史。fork 没有那条功能，合入会改错断开路径。本次未逐行重读 diff。 | 同 main |
| [`41bf2e7`](https://github.com/yituorou/meatshell/commit/41bf2e7dc3b21ab2fe1d62269518a192e7927f84) | 2026-10-10 | feat(terminal): render SGR 4 underline, including on blank cells (#444) | 新功能/分支不适用。绘制 SGR 4 下划线。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`4052010`](https://github.com/yituorou/meatshell/commit/4052010f2d9c3a6e64bdd3cdb49d67d71144c678) | 2026-10-10 | fix(terminal): draw glyphs missing from the terminal font with a system fallback (#461) | 未审。缺字回退，会改构建脚本。未逐行。 | 同 main |
| [`f01154b`](https://github.com/yituorou/meatshell/commit/f01154b56447898b9b9e609fc93521ba5f9da123) | 2026-10-11 | feat(terminal): per-session character encoding for Telnet and serial (#435) | 新功能/分支不适用。Telnet/串口每会话编码。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`9668248`](https://github.com/yituorou/meatshell/commit/9668248b38cfef935926c101c2e6348228a80d97) | 2026-10-11 | - **Telnet 与串口会话支持按会话设置字符编码（#435）。** 此前编码选项（#338）仅对 SSH 生效，通过 Telnet 或串口连接的 GBK 设备始终按 UTF-8 解码而显示乱码。现在 Telnet 与串口会按会话所选编码解码输出、编码输入，多字节字符跨读取被拆分时也不再显示为替换字符。会话编辑器在 SSH / Telnet（高级选项）和串口（主设置区）中提供编码选择，并新增 GB18030、EUC-JP 及西里尔 / 中欧等编码。 - **Per-session character encoding for Telnet and serial sessions (#435).** The encoding option (#338) only applied to SSH, so GBK equipment reached over Telnet or a serial line always decoded as UTF-8 and showed garbled text. Telnet and serial now decode output and encode input with the session's encoding, and multibyte characters split across reads no longer turn into replacement characters. The session editor offers the encoding picker for SSH / Telnet (Advanced) and serial (main section), with more encodings including GB18030, EUC-JP and Cyrillic / Central European code pages. | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`c473010`](https://github.com/yituorou/meatshell/commit/c473010599231fd7379f1a2ce0a0de2e2c2065c3) | 2026-10-14 | feat: add verified in-app Windows MSI updates | 新功能/分支不适用。应用内 Windows MSI 升级。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`a83eb4d`](https://github.com/yituorou/meatshell/commit/a83eb4d6cfaf3faa3c3347b7ee33bd6d1a311a15) | 2026-10-14 | test: exercise real Windows MSI upgrades and rollback | 新功能/分支不适用。MSI 升级测试，依赖上一笔更新器。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`c1be65b`](https://github.com/yituorou/meatshell/commit/c1be65b1a801ac3794e0ef4ecc59c079e0e7b646) | 2026-10-14 | docs: clarify Windows updater compatibility and validation scope | 新功能/分支不适用。Windows 更新器文档。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`d0dd681`](https://github.com/yituorou/meatshell/commit/d0dd68135feed33d88d4e9dccb2cf2a5aa53d90c) | 2026-10-14 | docs: record updater test environment and verified outcomes | 新功能/分支不适用。更新器测试记录。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`3818525`](https://github.com/yituorou/meatshell/commit/3818525b1177f5cd19fed07f19244a1a35942b47) | 2026-10-17 | feat(ui): launch terminals and AI agents from a new-tab menu (#427) | 新功能/分支不适用。新标签菜单启动终端和 AI agent。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`0bb9829`](https://github.com/yituorou/meatshell/commit/0bb982926561ea391cad79a2fa50c561f226a9c5) | 2026-10-17 | feat(ui): add custom AI agents for CLIs that are not detected (#427) | 新功能/分支不适用。自定义 AI agent。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`190d2de`](https://github.com/yituorou/meatshell/commit/190d2de7c9bb2f98ac8416b150088815ee409be7) | 2026-10-17 | feat(sftp): let the SFTP panel work as root via sudo (#438) | 新功能/分支不适用。SFTP 经 sudo 以 root 工作。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`2b43646`](https://github.com/yituorou/meatshell/commit/2b43646efa6520eaad7a03ba0038c881a7d691a3) | 2026-10-17 | feat(sftp): double-click opens text files in the built-in editor (#493) | 新功能/分支不适用。双击用内置编辑器打开文本。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`9bd395d`](https://github.com/yituorou/meatshell/commit/9bd395dfb44a3dcee8620a8f927488b5b279dc38) | 2026-10-18 | feat(editor): optionally dock the file editor in the main window (#494) | 新功能/分支不适用。编辑器停靠主窗口。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`3614b3a`](https://github.com/yituorou/meatshell/commit/3614b3a12e3a5d5ddc0d9467d2f29362ad5c96de) | 2026-10-18 | feat(editor): open files as editor tabs instead of a docked column (#494) | 新功能/分支不适用。编辑器改为标签。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`2192977`](https://github.com/yituorou/meatshell/commit/219297745dc201ec68ed0c60757b013dc83296da) | 2026-10-18 | fix(ssh): stop "__ms7: command not found" in tmux and nested shells (#496) | 缺陷属实待修。未登记。两侧 PROMPT_BODY 仍是 PROMPT_COMMAND="__ms7。子 shell 会报 command not found。未登记议题。 | 同 main |
| [`34d417b`](https://github.com/yituorou/meatshell/commit/34d417beeb0b3b70d38e11d6e4413a02c35541b6) | 2026-10-18 | fix(mcp): accept tool arguments sent as a JSON-encoded string | 已审明确拒绝。同一提交把 Slint 从 1.16.1 升到 1.18.1，并改约 600 个 vendor 文件。MCP 参数函数本身很小，但禁止整提交合入，也不升 Slint。slim 另外没有 MCP。 | 同 main |
| [`09ad492`](https://github.com/yituorou/meatshell/commit/09ad49267912a30b36169f9b979e5e238994d52d) | 2026-10-18 | feat(update): one-click updates for portable, AppImage, tarball and macOS builds (#390) | 新功能/分支不适用。便携/AppImage/macOS 一键更新。必须仍指向 woncc。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`4493937`](https://github.com/yituorou/meatshell/commit/44939373d39257644ca9cb3232a2920771470dc1) | 2026-10-18 | docs(changelog): add 0.7.8 release notes docs(changelog): 添加 0.7.8 版本更新日志 | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`ada3173`](https://github.com/yituorou/meatshell/commit/ada3173323a8e496d99ce57c16be33949338ea87) | 2026-10-18 | chore(release): 0.7.8 | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`b5af6d4`](https://github.com/yituorou/meatshell/commit/b5af6d4e1b95e8d4ecc4fb7c0d527d836c6eed5f) | 2026-10-18 | ci(release): name .deb packages like the other release files | 未审。deb 包文件名。未逐行，不是缺陷结论。 | 同 main |
| [`e14db52`](https://github.com/yituorou/meatshell/commit/e14db520e740b75655425e45eefd6d653a54e013) | 2026-10-18 | Add English issue templates alongside Chinese ones | 新功能/分支不适用。英文议题模板，不是缺陷。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`50a3f8d`](https://github.com/yituorou/meatshell/commit/50a3f8d2ef69e62829849f8d2111a6628e07c7af) | 2026-10-19 | feat(terminal): Alt+Enter toggles terminal fullscreen (#495) | 新功能/分支不适用。Alt+Enter 终端全屏。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`30088a9`](https://github.com/yituorou/meatshell/commit/30088a921ddfb870fabad36ee7a6a1cc58af70c1) | 2026-10-19 | feat(session): selectable terminal type per session (#498) | 新功能/分支不适用。每会话终端类型。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`cce71a9`](https://github.com/yituorou/meatshell/commit/cce71a980eb1ba7e40a66efc63e03cae8a6008f4) | 2026-10-19 | feat(sftp): Shift+click range multi-select in the file list | 新功能/分支不适用。Shift+点击范围多选。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`4b0c773`](https://github.com/yituorou/meatshell/commit/4b0c773a584767d7545079b888a6549345c16a85) | 2026-10-19 | feat(shortcuts): customisable keyboard shortcuts | 新功能/分支不适用。自定义快捷键。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`50fd2e5`](https://github.com/yituorou/meatshell/commit/50fd2e53a3f5ee7816ff3d64110594101fa15334) | 2026-10-19 | fix(ui): stop idle redraw loop after transfers (#499) | 缺陷属实待修。未登记。breathe-layer 仍是 animate opacity { iteration-count: -1 }。传输结束后会持续要帧。未登记议题。 | 同 main |
| [`022ad94`](https://github.com/yituorou/meatshell/commit/022ad9412c8cbddbefba15d2cd208dc245d4a49d) | 2026-10-19 | feat(shortcuts): list Ctrl+C as a rebindable Interrupt action | 新功能/分支不适用。把 Ctrl+C 列为可重绑定中断。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`e3f1f9b`](https://github.com/yituorou/meatshell/commit/e3f1f9b786efede655649f3dad6746a9412e1fee) | 2026-10-19 | feat(sftp): path bookmarks and recent folders (#500) | 新功能/分支不适用。路径书签。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`255ec1d`](https://github.com/yituorou/meatshell/commit/255ec1da5ade32cd12aa72bd645bdc70e19f0ae6) | 2026-10-19 | feat(update): prefer the .dmg for macOS one-click updates, fall back to .zip (#503) | 新功能/分支不适用。macOS 更新优先 dmg。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`5069374`](https://github.com/yituorou/meatshell/commit/5069374c0fffddf0fd4b30ad4a649a4a82a2eb63) | 2026-10-19 | feat(sftp): show drag-out copies in the Transfers panel (#504) | 新功能/分支不适用。拖出复制进传输面板。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`ba9e620`](https://github.com/yituorou/meatshell/commit/ba9e6208c3128692bb48f024e468078f1f799e13) | 2026-10-19 | chore(release): v0.7.9 | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`2c366c3`](https://github.com/yituorou/meatshell/commit/2c366c3eff8dbf0fcdde87708ead1fcb9d0ca7c1) | 2026-10-20 | feat(update): one-click updates for Linux .deb, /usr/local, Flatpak and AUR installs | 新功能/分支不适用。Linux 一键更新，含 slim 已删除的 AUR，且不能安装上游发布包。未逐行，未登记。 | 同 main |
| [`1cc51d1`](https://github.com/yituorou/meatshell/commit/1cc51d1a2db4299b2979f3f653c17a67cff0bb0d) | 2026-10-20 | feat(sftp): drag files out to Finder on macOS (#504) | 新功能/分支不适用。从 SFTP 拖出到 Finder。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`0251a0a`](https://github.com/yituorou/meatshell/commit/0251a0a90741f4c682d57c3cd73e8766d783ac8d) | 2026-10-22 | feat(update): replace update banner with toolbar icon + red-dot badge | 未审。更新入口改成工具栏红点。未逐行。已知 fork 工具栏已有专注、眼睛、图钉，但本笔 diff 未逐行，故不记成已审拒绝。 | 同 main |
| [`a1c69b8`](https://github.com/yituorou/meatshell/commit/a1c69b8c63fe5c2c34fd867c5757088c96fadad7) | 2026-10-22 | fix(update): use release manifest to avoid anonymous API limits | 已审明确拒绝。用 release manifest 替换更新检查。fork 仍请求 api.github.com/repos/woncc/meatshell/releases/latest。整段套用会换掉这个地址。没有把匿名 API 限额记成已发生的故障。 | 同 main |
| [`81acdf3`](https://github.com/yituorou/meatshell/commit/81acdf3f40ee78728aaf8a59c110a62879d4037f) | 2026-10-22 | feat(update): download mirrors with speed test for mainland China users | 新功能/分支不适用。下载镜像测速。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`2243a30`](https://github.com/yituorou/meatshell/commit/2243a30943be076961494a76a0157c264a301d60) | 2026-10-22 | feat(update): download mirrors with speed test and source picker for mainland China | 新功能/分支不适用。镜像源选择。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`35fe116`](https://github.com/yituorou/meatshell/commit/35fe116951063896846c97585fa20d7963aad359) | 2026-10-22 | fix(terminal): size columns to the terminal area when SFTP is docked left/right (#505) | 缺陷属实待修。未登记。term-cols 仍按 root.width 计算。SFTP 左右停靠时列数不对。未登记议题。 | 同 main |
| [`c1df7b2`](https://github.com/yituorou/meatshell/commit/c1df7b2be0441a7ce87a315c4f570a9aa4251b86) | 2026-10-23 | fix(session): save the picked jump host (#509) | 缺陷属实待修。未登记。跳板弹层先 close() 再回调，选择保存为空。未登记议题。 | 同 main |
| [`820c58a`](https://github.com/yituorou/meatshell/commit/820c58ab1d4d0f98d3f8d769ccf100d6571a087c) | 2026-10-23 | fix(rdp): stop mstsc reconnecting when the window is resized (#511) | 缺陷属实待修。未登记。.rdp 未写 dynamic resolution:i:0，调窗口时 mstsc 可能重连。未登记议题。不改密码存储。 | 同 main |
| [`135ab05`](https://github.com/yituorou/meatshell/commit/135ab058e8e79c8457cb9de497923f8a403ee6fe) | 2026-10-23 | fix(rdp): save the password in Credential Manager instead of the .rdp file | 已审明确拒绝。把 RDP 密码从 DPAPI 的 .rdp 改到凭据管理器。fork 的 rdp.rs 仍写 password 51:b:。存储方式保持不动。 | 同 main |
| [`7da8240`](https://github.com/yituorou/meatshell/commit/7da82408eae1a30d812dd9df933552c7d187b4f7) | 2026-10-23 | chore(release): v0.8.0 chack CHANGELOG.md yourself | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`0560292`](https://github.com/yituorou/meatshell/commit/0560292b069285d28f91e14d1ebd0ee0b9867a17) | 2026-10-23 | feat(config): import FinalShell local, remote, and SOCKS5 port forwards | 新功能/分支不适用。导入 FinalShell 端口转发。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`2713e5d`](https://github.com/yituorou/meatshell/commit/2713e5d61e0ee1709c3634b8577746c60f608926) | 2026-10-26 | fix(mcp): accept saved_session_id alias; bridges drop session_id | 未审。main：automation/mcp 约 600 行，未逐行。slim：MCP 与 automation 已删除，分支不适用。 | 新功能/分支不适用。main：automation/mcp 约 600 行，未逐行。slim：MCP 与 automation 已删除，分支不适用。 |
| [`e704225`](https://github.com/yituorou/meatshell/commit/e704225049c22ce29a03c08b57f7097a3e6185ef) | 2026-10-26 | feat(tabs): add a local tab's ssh connection to the session list (#501) | 新功能/分支不适用。把本地标签的 SSH 连接写入会话列表。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`b6553bf`](https://github.com/yituorou/meatshell/commit/b6553bffdfb1e4c6cd192b64597b81cc6705322c) | 2026-10-26 | feat(settings): color theme presets and an immersive colors switch | 新功能/分支不适用。主题预设。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`5c39ca0`](https://github.com/yituorou/meatshell/commit/5c39ca0f8b94f4013ded75e3615dbcb6cdedbdf6) | 2026-10-26 | feat(telnet): save a username and password and log in automatically (#507) | 新功能/分支不适用。Telnet 保存账号并自动登录。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`459e98f`](https://github.com/yituorou/meatshell/commit/459e98ff9caa8b9815ea649f44a8c0cc5a7750df) | 2026-10-26 | feat(sftp): paste copied files and folders to upload them (#508) | 新功能/分支不适用。粘贴文件以上传。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`34cdc18`](https://github.com/yituorou/meatshell/commit/34cdc184add6219fd42b0e449b09767d4ad091ef) | 2026-10-27 | feat(sftp): split local/remote view with drag-and-drop transfers | 新功能/分支不适用。SFTP 左右分屏。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`9c9d701`](https://github.com/yituorou/meatshell/commit/9c9d7011bd45f63d18fbfb631ec7dc1e22c70384) | 2026-10-29 | fix(ssh): restore directory following after sudo | 未审。sudo 后目录跟随，约 800 行。未逐行。 | 同 main |
| [`cdf47c7`](https://github.com/yituorou/meatshell/commit/cdf47c7186a9ab39a39be24d20e00b2370d7dde4) | 2026-10-30 | feat(mcp): show a copyable client config in Settings › MCP | 新功能/分支不适用。设置页展示 MCP 客户端配置。slim 已删除 MCP。main 也不把这项当缺陷。未逐行，未登记。 | 同 main |
| [`0a8e4a3`](https://github.com/yituorou/meatshell/commit/0a8e4a32a438d1ff9ddcdf54e3e95974947b702d) | 2026-10-30 | fix(sftp): restore paste-to-upload after merging the split view (#508, #520) | 新功能/分支不适用。修的是 fork 没有的分屏、粘贴上传或每会话 FreeRDP 选项。1ea211c 已确认没有 src/app/sftp_local.rs。未登记。 | 同 main |
| [`af41df3`](https://github.com/yituorou/meatshell/commit/af41df33f29558d3ab848af195cd45f81508238f) | 2026-10-30 | fix(ssh): give zsh default Home/End/PageUp/PageDown/Delete bindings (#519) | 未审。提交说明是向远端 zsh 注入 Home/End 等绑定。diff 未逐行。 | 同 main |
| [`790633a`](https://github.com/yituorou/meatshell/commit/790633a8aaee27f25cf595b16d7df88155703cbe) | 2026-10-30 | fix(ui): keep macOS traffic lights off detached windows' content and use the normal cursor over panel empty space (#360) | 未审。macOS 文件列表滚动或交通灯。未逐行。 | 同 main |
| [`8163a2c`](https://github.com/yituorou/meatshell/commit/8163a2ce24d143c10a3992d86125ef6b86ad5ff8) | 2026-10-30 | feat(ssh): configurable SSH keepalive in Settings › Connection (#515) | 新功能/分支不适用。可配置 SSH keepalive。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`6f8403d`](https://github.com/yituorou/meatshell/commit/6f8403d89377b548c96a044d27406e05c4cd5dea) | 2026-10-30 | fix(build): compile headless builds again | 已审明确拒绝。只为 headless 编译 update_mirrors。fork 没有镜像列表，slim 也没有 headless。已对过补丁和 Cargo 功能。 | 同 main |
| [`b7fd33f`](https://github.com/yituorou/meatshell/commit/b7fd33f3bf6f02510cdd425b29740fa7ab26e186) | 2026-10-30 | chore(release): v0.8.1 | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`97fffb8`](https://github.com/yituorou/meatshell/commit/97fffb89a2e45c25ccfefeebadad6d4aed4ba563) | 2026-10-30 | 删除一个不小心放上去的垃圾文件 Delete a file that was added by mistake | 已审明确拒绝。删除上游误加的 output/covers 图片。fork 历史里没有该文件。 | 同 main |
| [`91fffa7`](https://github.com/yituorou/meatshell/commit/91fffa790f712906e01c322e7cb09327161a50fa) | 2026-10-26 | fix(macos): use `open` instead of `xdg-open` to reveal files | 缺陷属实待修。未登记。更新页和仓库链接已经用 open。下载目录与 SFTP open_with_os 在非 Windows 上仍调用 xdg-open。未登记议题。 | 同 main |
| [`7f98487`](https://github.com/yituorou/meatshell/commit/7f98487f7c987252a4e064c17b3f1eecb2771753) | 2026-11-02 | feat(rdp): add per-session FreeRDP options and bilingual arguments | 新功能/分支不适用。每会话 FreeRDP 参数。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`cf385aa`](https://github.com/yituorou/meatshell/commit/cf385aa192dcc70fada94f64a5b9e71ed3fe2343) | 2026-11-03 | fix(ssh): don't freeze the terminal when shell-integration setup never completes (#524) | 缺陷属实待修。未登记。没有 PROMPT_SETUP_TIMEOUT。shell 集成完成标记不来时，后续输出会一直缓冲。未登记议题。 | 同 main |
| [`5a2d98a`](https://github.com/yituorou/meatshell/commit/5a2d98a1ee172a8673533d79a7a5f69b645396dc) | 2026-11-03 | test(rdp): fix Windows test build after FreeRDP options (#530) | 新功能/分支不适用。修的是 fork 没有的分屏、粘贴上传或每会话 FreeRDP 选项。1ea211c 已确认没有 src/app/sftp_local.rs。未登记。 | 同 main |
| [`9ffc9af`](https://github.com/yituorou/meatshell/commit/9ffc9af7907980ffc1f4de7fe3c80a27973926a8) | 2026-11-03 | perf(terminal): smooth rendering and lower memory under heavy log output | 新功能/分支不适用。大日志下的渲染性能。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`dae4b1d`](https://github.com/yituorou/meatshell/commit/dae4b1d724e01ea75813d95099cd6c3185d3e6c5) | 2026-11-05 | fix(ssh): wait for the shell prompt before injecting shell integration (#524) | 未审。与 shell 集成注入的后续改动有关。当前注入行 877 字节。diff 未逐行，不记成已审拒绝，也不记成已证实缺陷。 | 同 main |
| [`0e47f1c`](https://github.com/yituorou/meatshell/commit/0e47f1c84f32d423ee51c12c135640f8063f5ec4) | 2026-11-05 | fix(session): keep the dialog buttons visible with advanced options open (#525) | 未审。会话框按钮与页脚测试。未逐行。fork 对话框另有脱敏行。 | 同 main |
| [`191885f`](https://github.com/yituorou/meatshell/commit/191885f58a4e397c6dc426478681140011655b92) | 2026-11-05 | test(session): follow the pinned dialog footer in coordinate-based tests (#525) | 未审。会话框按钮与页脚测试。未逐行。fork 对话框另有脱敏行。 | 同 main |
| [`f5727cf`](https://github.com/yituorou/meatshell/commit/f5727cf10b0b1b39b890c9b80919922ae7df973e) | 2026-11-05 | fix(sftp): delete every checked entry, add Del key, show Modified sooner (#531) | 新功能/分支不适用。勾选删除、Del 键和更早显示修改时间。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`b845a8f`](https://github.com/yituorou/meatshell/commit/b845a8ff588d1bb4509f0663dde88b1c0dc13c07) | 2026-11-05 | fix(serial): stop keystrokes lagging and replaying after release (#534) | 未审。串口输入重写约 300 行。未逐行。 | 同 main |
| [`f0e8df2`](https://github.com/yituorou/meatshell/commit/f0e8df251be9ca5cf6b50e956c653852c4ab20b3) | 2026-11-05 | fix(quick-commands): a group typed as "default" no longer duplicates it (#534) | 缺陷属实待修。未登记。快捷命令把字面量 default 存成另一个分组。没有 normalize_group。未登记议题。 | 同 main |
| [`ae8b05c`](https://github.com/yituorou/meatshell/commit/ae8b05cb76db38b019b687821989bfdef8162b2a) | 2026-11-05 | feat(terminal): session log from the tab menu, find count and navigation (#535) | 新功能/分支不适用。标签菜单会话日志和查找计数。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`6d5bf4c`](https://github.com/yituorou/meatshell/commit/6d5bf4c68b812f9a5ee8dbb58cfb0960b7f0b081) | 2026-11-05 | fix(windows): don't block startup on a slow or broken wsl.exe (#537) | 缺陷属实待修。未登记。启动时在界面线程同步执行 wsl.exe --status。未登记议题。 | 同 main |
| [`dd1ecb8`](https://github.com/yituorou/meatshell/commit/dd1ecb8c9358ac85d767233db5881596d9be1138) | 2026-11-05 | fix(ssh): keep the shell-integration line under macOS's 1024-byte tty limit (#541) | 未审。与 shell 集成注入的后续改动有关。当前注入行 877 字节。diff 未逐行，不记成已审拒绝，也不记成已证实缺陷。 | 同 main |
| [`1ea211c`](https://github.com/yituorou/meatshell/commit/1ea211c1a8c91ed4145c9ff10caf8693cf4ba756) | 2026-11-07 | fix(sftp): never probe local paths on the UI thread, stop copy loops | 新功能/分支不适用。修的是 fork 没有的分屏、粘贴上传或每会话 FreeRDP 选项。1ea211c 已确认没有 src/app/sftp_local.rs。未登记。 | 同 main |
| [`92fb04c`](https://github.com/yituorou/meatshell/commit/92fb04c4f7b3e6ecc948723af42a38fbca6a0db1) | 2026-11-07 | feat(sftp): remember the split view's state, folder and divider; add local places | 新功能/分支不适用。分屏状态记忆。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`fe52332`](https://github.com/yituorou/meatshell/commit/fe523325e903e52c1b9bbe55d7f6a3f1d789fd5a) | 2026-11-07 | feat(sftp): bookmarks and recent folders for the local pane | 新功能/分支不适用。本地窗格书签。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`ab4241b`](https://github.com/yituorou/meatshell/commit/ab4241b39d8c58c665c2fb22c8899a0b52f82645) | 2026-11-08 | fix(macos): let the SFTP file list scroll under the file names (#540) | 未审。macOS 文件列表滚动或交通灯。未逐行。 | 同 main |
| [`b22c9cd`](https://github.com/yituorou/meatshell/commit/b22c9cd73c51c1aaa103cc57921661294ba3952f) | 2026-11-08 | Fix terminal input and login-script issues (#542) | 未审。提交说明是修饰键序列和登录脚本注入。diff 未逐行，不记成已审拒绝。 | 同 main |
| [`1f62803`](https://github.com/yituorou/meatshell/commit/1f628033ac8008765a4bcea1d6d2e953c647b101) | 2026-11-08 | Enable VT100 line drawing by default (#543) | 未审。提交说明是默认打开 VT100 线绘。diff 未逐行。 | 同 main |
| [`6f09269`](https://github.com/yituorou/meatshell/commit/6f0926979ed945cd31565cf9830d4b933cb47f38) | 2026-11-08 | fix(sftp): restore split-view bindings lost in a merge (#532) | 新功能/分支不适用。修的是 fork 没有的分屏、粘贴上传或每会话 FreeRDP 选项。1ea211c 已确认没有 src/app/sftp_local.rs。未登记。 | 同 main |
| [`096baf5`](https://github.com/yituorou/meatshell/commit/096baf58cf8fde98ebb02d7b2138935155a0671b) | 2026-11-08 | chore(release): v0.8.2 | 已审明确拒绝。上游版本号或 changelog。fork 版本是 0.7.12 与 0.7.12-slim.1，不能覆盖。 | 同 main |
| [`b343829`](https://github.com/yituorou/meatshell/commit/b343829ae476a672c3eb1f036c69a9c873e1968e) | 2026-11-08 | ci(release): build macOS on macos-15, macos-14 was retired | 缺陷属实待修。未登记。两侧 release.yml 仍写 macos-14。该提交说明 runner 已退役且作业失败。未登记议题。只应改 runner，不能替换整份发版工作流。 | 同 main |
| [`286e0c1`](https://github.com/yituorou/meatshell/commit/286e0c1b196c06f16f7db66386126d8086b41082) | 2026-11-08 | fix(sftp): ask before overwriting an existing remote file on upload | 未审。上传前询问覆盖。未逐行，不把“没有这句确认”记成已证实缺陷。 | 同 main |
| [`a9a3a64`](https://github.com/yituorou/meatshell/commit/a9a3a644e86b441cfb6be0c91824fb529043a786) | 2026-11-08 | feat(sftp): hidden-files toggle, sortable headers and a name filter for the local pane | 新功能/分支不适用。本地窗格隐藏文件、排序和过滤。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |
| [`ef7bcda`](https://github.com/yituorou/meatshell/commit/ef7bcda809539e451129ddab990ebc5ab09d0090) | 2026-11-08 | feat(sftp): remove single recent entries, reorder bookmarks, open the folder | 新功能/分支不适用。SFTP 最近位置整理。 依据提交说明归为新功能或产品行为，未逐行读 diff，未登记。不是已审拒绝。 | 同 main |

## 未审的合并提交

基线到分类清单终点还有 29 个合并提交。它们没有逐个打开，类别是未审，也不计入上面的 98，更不是 29 个缺陷。

- [`6b28b0e`](https://github.com/yituorou/meatshell/commit/6b28b0e00c12c936c0c1e87e939194e426a6d5dc) Merge pull request #552 from qux-bbb/feat/sftp-places-polish
- [`257a1d7`](https://github.com/yituorou/meatshell/commit/257a1d77794e2057dc20dffe62b6b1c38311240f) Merge pull request #551 from qux-bbb/feat/sftp-local-list-options
- [`a6fddb1`](https://github.com/yituorou/meatshell/commit/a6fddb13d3e5741ae9f6a68e5aeb17380db78305) Merge branch 'main' into feat/sftp-local-list-options
- [`ba74638`](https://github.com/yituorou/meatshell/commit/ba74638272ab245a7c847802e855a0e6ff5381fd) Merge pull request #550 from qux-bbb/feat/sftp-upload-conflict
- [`adbf0f5`](https://github.com/yituorou/meatshell/commit/adbf0f5ac56ab8326e97495ccefff97290946d90) Merge remote-tracking branch 'origin/main'
- [`9241500`](https://github.com/yituorou/meatshell/commit/924150014ec6c4026486318934fbb66ef652ef59) Merge pull request #548 from qux-bbb/feat/sftp-local-places
- [`0648594`](https://github.com/yituorou/meatshell/commit/064859498a0ec022d43749ab355445259488e5d0) Merge remote-tracking branch 'upstream/main' into feat/sftp-local-places
- [`ae78515`](https://github.com/yituorou/meatshell/commit/ae7851516f29640ab54d15342c48e056c87f72f9) Merge remote-tracking branch 'origin/main'
- [`f62e8ee`](https://github.com/yituorou/meatshell/commit/f62e8ee04e126769cd614be9cb5a3a098c25b640) Merge branch 'main' into feat/sftp-local-places
- [`4140d3c`](https://github.com/yituorou/meatshell/commit/4140d3c950edf02249b6838644d1653078442ec0) Merge pull request #547 from qux-bbb/feat/sftp-split-memory
- [`d49c60a`](https://github.com/yituorou/meatshell/commit/d49c60a78aa174a71f9639ce84fd6c0ae0440bce) Merge branch 'main' into feat/sftp-split-memory
- [`d8c5822`](https://github.com/yituorou/meatshell/commit/d8c58229fc6676eee6307c1837a207189a1b845e) Merge pull request #546 from qux-bbb/fix/sftp-local-robustness
- [`4b92c9d`](https://github.com/yituorou/meatshell/commit/4b92c9d4c716011a8d78c0dce7aaf6637ddb73f4) Merge pull request #530 from andnnl/feat/rdp-freerdp-options
- [`a142126`](https://github.com/yituorou/meatshell/commit/a14212636f66f65524fc7a4ed66f60ee25e56be9) Merge pull request #517 from qingbinzeng/fix/macos-open-download-dir
- [`368c637`](https://github.com/yituorou/meatshell/commit/368c6379082d8796633f7962a9cecafea4a7681f) Merge remote-tracking branch 'origin/main'
- [`a941b6d`](https://github.com/yituorou/meatshell/commit/a941b6d44a50f59a1c2bdb936fe97f432779e7a7) Merge pull request #522 from old-rubick/fix/513-sudo-cwd
- [`03dcb3a`](https://github.com/yituorou/meatshell/commit/03dcb3a568c84ff3bd39d7977da3eb8f7043020e) Merge remote-tracking branch 'origin/main'
- [`29c49e4`](https://github.com/yituorou/meatshell/commit/29c49e4251d42130b63e07c8df960e4abfea1893) Merge pull request #520 from qux-bbb/feat/sftp-split-view
- [`f12b5d7`](https://github.com/yituorou/meatshell/commit/f12b5d77fe1c56d40c2c168afe0ae2511b5c49de) Merge pull request #514 from Venshao/feat/finalshell-port-forward-import
- [`5ae49e3`](https://github.com/yituorou/meatshell/commit/5ae49e3027bb2f2b6daa2a46bd732d33e917f97b) Merge pull request #512 from yanmuq/fix/update-release-rate-limit
- [`19f5956`](https://github.com/yituorou/meatshell/commit/19f59561c111380197c6382d594314c59aeb6625) Merge remote-tracking branch 'origin/main'
- [`98ca51a`](https://github.com/yituorou/meatshell/commit/98ca51ab09c96b924fb931b41e45f92ff9c9ba9d) Merge pull request #497 from yanmuq/feat/windows-in-app-update
- [`cf8dc81`](https://github.com/yituorou/meatshell/commit/cf8dc81a8f81b2b453233d2c935ff62faac2286a) Merge remote-tracking branch 'origin/main'
- [`a0e09e6`](https://github.com/yituorou/meatshell/commit/a0e09e619e23a9f4293eb3dec0da3c3dbcdffb61) Merge pull request #491 from nilxbit/dev3
- [`6dcdfb4`](https://github.com/yituorou/meatshell/commit/6dcdfb4e914c95645c93ca39c3934c817ef8ffd6) Merge remote-tracking branch 'origin/main'
- [`09999fc`](https://github.com/yituorou/meatshell/commit/09999fc9a9da3b69ebd8aebadd4784b3cb72a1a4) Merge pull request #489 from nilxbit/dev2
- [`da95724`](https://github.com/yituorou/meatshell/commit/da9572427b190c18002c9a78ace17454f34edd3a) Merge pull request #487 from nilxbit/dev1
- [`29aa933`](https://github.com/yituorou/meatshell/commit/29aa933364827c8e59c21e8caf0f11bcf32d0816) Merge remote-tracking branch 'origin/main'
- [`af4fae6`](https://github.com/yituorou/meatshell/commit/af4fae61d4288f9a41715a93f3729ab522eff728) Merge pull request #485 from lyj404/main

## 维护

1. `git fetch upstream main` 之后，只看 `6b28b0e00c12c936c0c1e87e939194e426a6d5dc..upstream/main`。新提交先标未审。审完才改清单终点。
2. 表内旧行不因上游新增而整表重审。只有 `main` 或 `slim` 上相关代码变了，或回归测试失败，才复查那一行。
3. 某行真的合入之后，把类别改成已吸收，并写上 fork 提交。不要用清单终点代替这一列。
4. 不整包合并上游，不升 Slint，不改本 fork 版本号，不把一键更新指向上游安装包。`slim` 不恢复已删除的 MCP、CLI、automation、headless、AUR。

## 客户端验收

Windows 和 macOS 实机点选还没有做。这不改变「已吸收」代码已经在对应分支上的事实，也不把「缺陷属实待修」收成已修好。

