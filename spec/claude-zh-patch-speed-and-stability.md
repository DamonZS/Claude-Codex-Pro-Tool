# Claude 一键汉化：提速与稳定性修复

## 背景

当前实现位于 `crates/claude-codex-pro-core/src/claude_zh_patch.rs`（3,397 行、130 个函数），对 MSIX 版 Claude Desktop（本机 2.42304.2.0）做整包汉化：写 3 份 i18n JSON、遍历 `app/resources/ion-dist/assets/**` 下**全部** `.js`（本机实测 3,589 个 / 115.94 MB）逐块判断是否需要替换文案、改写并备份到 `%LOCALAPPDATA%\Claude-zh-CN-official-backup`（本机实测 5,193 个文件 / 498.2 MB，自 2026-06-16 起从未清理），最后写 `%APPDATA%\Claude-3p\config.json` 的 `locale` 并清渲染缓存。

用户反馈（m46503）：一键汉化耗时过长，并且可能导致 Claude Desktop 不稳定、崩溃甚至无法打开。

本次排查到的具体原因：

1. **文案扫描为 O(文本长度 × 词表)**：`chunk_needs_or_has_ui_patch` 对每个块执行约 460 次全文 `contains`（`zh_text_pairs` ≈ 230 组，每组中文/英文两种引号变体），外加一次 `patch_locale_arrays` 全文拷贝。按 116 MB × 460 遍估算约 50 GB 扫描量，这是"时间过长"的主因。
2. **同一文件被读 3 次**：调用方读一次、`patch_chunk` 内部再读一次、改动后为避免比较错误再读一次。
3. **`node --check` 每个改动块各起一个进程**。
4. **备份从不清理**：`restore_patch_at` 会把备份目录里**全部**条目写回当前安装，包含旧版本 Claude 的旧 chunk——既慢，又会把陈旧文件塞进新安装。
5. **ACL 提权（takeown/icacls）改动 WindowsApps 权限后不恢复**，失败路径无回滚；安装成功但 Claude 启动失败时也没有自动回滚，用户可能停在"半汉化 + 打不开"的状态。
6. 无阶段耗时日志，线上无法量化，也无法证明优化有效。

## 目标

包含：

1. 文案扫描从"每块约 460 遍全文扫描"降为"每块一遍"。
2. 每块只读一次、必要时只写一次。
3. 安装时清理陈旧备份；恢复时只恢复当前安装里仍存在的目标。
4. 安装/恢复记录阶段耗时与计数日志。
5. 安装后 Claude 启动失败时自动回滚（恢复官方文件、移除 locale 配置），并明确告知用户。
6. 翻译词表、译文内容、备份命名、路径与现有 UI 行为保持不变。

不包含：修改词表或译文内容；支持非 MSIX 安装；引入新依赖；改前端界面；降低 `node --check` 的严格性。

## 用户视角

用户点「Claude 一键汉化」后：等待时间明显缩短（目标是从数分钟级降到数十秒级）；如果汉化后 Claude 没能启动，CCP 自动回滚到官方文件并提示"已回滚，Claude 可正常打开"，而不是留下一个打不开的状态。失败时日志里能查到各阶段耗时，便于定位。

## 功能要求

1. 新增私有 `NeedleMatcher`：由既有词表（`zh_text_pairs` 的英文、`embedded_chunk_patch_pairs` 的首元素、`zh_raw_patch_pairs` 的首元素，以及引号变体）在首次使用时构建 **Aho-Corasick automaton（`MatchKind::LeftmostFirst`）**，`contains_any(text)` 单趟判定是否含任一待替换词；判定结果必须与旧的逐词 `contains` 完全等价。
   - 备注（实测修正）：最初实现的"首字节桶 + 候选表"在真实 JS 上只快 3.3×——引号密集的代码在每个 `"` 处都要比对约 230 个候选，与旧算法同阶。改用 Aho-Corasick 后真机扫描 32.2 s → 2.97 s（10.9×），合成语料 3.50 s → 0.315 s（11.1×）。
2. `chunk_needs_or_has_ui_patch` 用 matcher 取代逐词扫描；语言数组判定改用 `text.contains("\"en-US\"")` 作为必要条件（`patch_locale_arrays` 只解析以 `"en-US"` 为首的双引号数组）。
3. `patch_chunk` 改为接收已读入的 `String` 并返回 `Option<String>`；`patch_chunks_in_parallel` 每块只读一次、必要时写一次。
4. 安装开始时清理备份：删除"解码后目标文件已不存在"的备份条目，清理数量写入日志与 timings。
5. `restore_patch_at` 只恢复目标文件仍存在的备份；恢复结束后同样清理陈旧备份。
6. `ClaudeZhPatchOutcome` 新增可选 `timings` 字段（`scannedChunks`、`patchedChunks`、`prunedBackups`、`scanMs`、`patchMs`、`validateMs`、`pruneMs`），serde camelCase 且默认空，向后兼容。
7. 管理端在安装完成处记录 `manager.claude_zh_patch.install.finish`（含 timings 与 changedFiles 数量）；Claude 启动失败时记录 `manager.claude_zh_patch.install.rollback`、调用 `restore_patch()`，并在返回文案中说明已回滚。

## 数据与接口

- 命令入参与返回结构不变；`ClaudeZhPatchOutcome` 仅新增可选字段。
- 备份文件命名（`encode_backup_name` / `decode_backup_name`）、locale 配置路径、i18n 资源路径均不变。

## 技术约束

- 仅新增一个直接依赖 `aho-corasick = "1.1"`（它本来就是 `regex` 的传递依赖，版本已在 `Cargo.lock` 中，不引入新的第三方代码面）；其余改动只用标准库。
- 改动集中在 `crates/claude-codex-pro-core/src/claude_zh_patch.rs` 与 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`。
- 不改变"改哪些文案、按什么顺序改"的语义。
- 保留既有错误文案、提权流程与备份目录结构。

## 变更记录

- 2026-11-04：按用户反馈（m46503）立项并实现阶段一。
  - 扫描闸门由"每块约 460 遍 `contains`"改为 Aho-Corasick 单趟；真机（Claude 2.42304.2.0，3,554 块 / 117.0 MB）实测 32.22 s → 2.97 s（10.9×）。
  - 每块只读一次；备份目录在安装开始与恢复结束各清理一次；恢复只写目标仍存在的备份，且恢复后清渲染缓存（避免缓存里继续跑旧补丁导致"恢复后仍打不开"）。
  - 安装结束时记录 `install.finish`（7 阶段耗时 + 计数）；未通过最终状态校验或 Claude 启动失败时自动回滚并记录 `install.rollback`。
  - 新增两个 `#[ignore]` 基准测试（合成语料 / 真实安装）与若干单测、一个契约测试。
  - 未做（保留在非目标）：`node --check` 每块一次进程（校验阶段已多线程，单块 75.8 ms）、ACL 权限回收、非 MSIX 安装。

## 交付范围

- core：`NeedleMatcher`、扫描/补丁/备份/恢复改造、`ClaudeZhPatchOutcome.timings`、单元测试。
- manager：`install.finish` 与 `install.rollback` 事件、启动失败自动回滚、契约测试。
- 文档：本 spec 与对应 acceptance。
- 不做：前端改动、词表扩充、安装策略变更。