# 概览 Agent 品牌图标与顶栏对齐验收

对应 spec/overview-agent-brands.md。

1. 总览 Agent 筛选实测字体为 13px，品牌图标为 16px，Agent 名称、筛选点击保持有效。
2. Agent 概览品牌容器显示 24px 品牌图标，claude/other 类型文本不再代替图标；未知品牌显示名称首字母。
3. 资源从仓库构建打包，实机已加载图标 complete 且 naturalWidth > 0，品牌与指定参考文件一致。
4. 顶栏 commandbar 跨满 grid，操作组右边缘与内容右边缘一致；1180/960 内容宽度下不与左侧控件重叠。
5. 类型检查、Vite/Release 构建、git diff --check 有真实日志；既有无关类型错误应单列报告，不修改无关文件。
6. BASELINE/MODIFIED/ROLLBACK 同输入验证已运行，回滚恢复原始哈希；四角色证据可重开。默认 target/release 保留最新应用。

非目标：新增 Agent、修改统计口径、后端和用户状态、提交或推送。
