# 验收标准：Claude 开发配置新增式写入

验证对象：`spec/claude-dev-mode-additive-profile-write.md`

## 验收项

1. 不覆盖 CC Switch 固定 profile
   - 通过标准：预置 `00000000-0000-4000-8000-000000157210.json` 和对应 meta 条目后执行写入，原文件内容及条目保持不变。

2. 新增独立 CCP profile
   - 通过标准：写入后生成不同于 CC Switch ID 的 CCP profile 文件，meta 追加该条目，`appliedId` 指向新 ID。

3. 同供应商幂等、不同供应商并存
   - 通过标准：相同名称和 Base URL 重复写入不增加重复条目；不同供应商新增不同文件和条目。

4. 不泄漏密钥到 ID
   - 通过标准：更换 API Key 不改变 profile ID，ID 和路径中不含 Key 文本。

5. 合并保留现有数据
   - 通过标准：普通配置、3P 配置、profile 和 `_meta.json` 的未知字段及其他条目均保留。

6. 空供应商与恢复操作不删除 profile
   - 通过标准：只开启开发模式外壳或切回官方部署模式时，已有 profile 文件和 meta 条目仍存在。

7. 错误回滚使用实际动态路径
   - 通过标准：预览、备份、快照、返回 payload 均指向本次供应商的动态 profile 路径。

8. UTF-8 BOM 配置兼容
   - 通过标准：带 UTF-8 BOM 的合法 `claude_desktop_config.json` 可以完成读取、字段合并和写回，既有字段不丢失。
   - 通过标准：真正损坏或根节点不是对象的 JSON 仍然拒绝写入，原文件保持不变。

9. 真实验证
   - 通过标准：以下检查通过：
     - `cargo test -p claude-codex-pro-core --test claude_desktop_provider -- --nocapture`
     - `cargo test -p claude-codex-pro-core plugin_hub -- --nocapture`
     - `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`
     - `npm --prefix apps/claude-codex-pro-manager run check`
     - `npm --prefix apps/claude-codex-pro-manager run vite:build`
     - `cargo fmt --all -- --check`
     - `cargo build -p claude-codex-pro-manager`

## 非验收范围

- 不要求本回合重启 Claude Desktop。
- 不使用真实 API Key 发起网络请求。
- 不删除当前用户已有配置。
