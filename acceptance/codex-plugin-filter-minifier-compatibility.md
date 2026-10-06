# Codex 插件过滤压缩名称兼容验收

对应 `spec/codex-plugin-filter-minifier-compatibility.md`。

- Node VM 执行生产过滤补丁：旧 `u/ne` 与新版 `Mj`、重命名参数/函数、空白差异均保留市场插件。
- 普通搜索、marketplaceName 搜索、带额外搜索条件的回调及 thisArg 语义保持原结果；无官方市场样本仍走原生过滤。
- 解锁关闭和现有禁用模式仍走原生过滤。
- 原版执行新版回调用例失败；修改版通过；独立副本还原后输出及退出码与原版一致，SHA256 恢复一致。
- `node scripts/test-plugin-build-flavor-filter.cjs` 与 `cargo test -p claude-codex-pro-core --test cdp_bridge` 通过，默认 Release 构建成功。
- 重新注入后实时 CDP 显示过滤补丁版本 14，新版渠道回调保留全部样本，搜索继续收窄；记录插件页实际条目。

不包括改变官方安装包、下载新插件或更改用户供应商配置。实机结论仅覆盖当前 Windows / Codex 26.930.4958。

## 本轮验证记录（2026-10-05）

- Node VM 行为测试：原版和独立回滚副本均 2 项通过、2 项失败，退出码 1；修改版 4 项通过，退出码 0。回滚后 SHA256 与原版一致，补丁重构与修改版一致。
- `node --check assets/inject/renderer-inject.js` 通过；core `cdp_bridge` 69 项通过；默认 manager Release 构建退出码 0。
- 重新注入后 CDP 现场结果：版本 `14`，原生过滤保留 2 个样本，补丁保留全部 5 个样本；搜索结果为 `Slides`，额外条件过滤结果为 1 个样本。
- 真实插件页 `Education & Research` 区域显示 `LaTeX` / `Compile LaTeX with TeX tools`，CDP 查询退出码 0。
- 七个指定管理页面已实机检查，均无顶部重复页面标题。
- 原始 BASELINE/MODIFIED/ROLLBACK 输出、哈希与现场补充记录见 `target/task-evidence/plugin-exit-20261005/plugin/VERIFICATION.txt`。
