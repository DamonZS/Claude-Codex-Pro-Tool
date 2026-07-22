# README 导航截图矩阵与能力说明验收标准

对应规格：`spec/readme-navigation-screenshot-matrix.md`

## 通过标准

- 中英文 README 均包含管理工具界面预览章节。
- 截图矩阵每个完整行包含四张图片，覆盖全部九个一级导航。
- 每张图片均有页面名称，且仓库中的相对路径真实存在。
- 中英文 README 的功能说明均包含 Codex 主题系统。
- 中英文 README 的功能说明均包含系统提示词与指令模板管理。
- 两版首屏均明确包含产品定位、支持平台、核心能力和下载入口。
- 两版均提供可点击的快速导航，并自然覆盖与真实能力对应的搜索关键词。
- 截图中未发现 API Key、Token、私密 Base URL、个人路径、账号信息或私密会话全文。
- 原有 README 主要安装和功能说明未被误删。

## 验证方式

- 人工检查所有截图内容和矩阵布局。
- 脚本检查 README 中 `docs/screenshots/` 图片引用均存在。
- 运行 `git diff --check` 检查空白与补丁格式。

## 完成证据

- README diff。
- README_EN diff。
- `docs/screenshots/` 图片清单。
- 图片引用检查和 `git diff --check` 输出。

## 非目标

- 不要求修改应用 UI 或重新构建应用。
- 不要求覆盖导航之外的每个二级弹窗和操作状态。
