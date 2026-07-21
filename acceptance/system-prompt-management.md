# 系统提示词管理验收标准

对应规格：`spec/system-prompt-management.md`

## 通过标准

- 左侧存在“系统提示词”一级入口；页面不显示重复路由标题带，打开后显示紧凑横向状态区、启用方式、分类筛选和每行三张模板卡片。
- 新增、编辑、导入 Markdown、删除和分类筛选可用，非法文件名、空标题和空内容被阻止。
- “替换原提示词”启用后，`config.toml` 的 `model_instructions_file` 指向 CCP 托管文件且文件内容等于模板内容。
- “保留原提示词”启用后，托管文件同时包含启用前指令内容与所选模板。
- 停用后恢复启用前的配置值；原本没有该键时删除该键。
- 删除当前模板被阻止，外部改写配置后停用不会覆盖外部配置。
- 修改 `config.toml` 前存在备份，写入采用同目录临时文件替换。
- GitHub Markdown 同步失败不会删除或修改已有本地模板。
- 不修改供应商、代理、模型映射、Codex 注入和 Claude 配置代码。

## 验证方式

```powershell
cargo test -p claude-codex-pro-core system_prompt -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
```

另需手动检查浅色、深色、960x640 和常用桌面分辨率下的文本溢出、滚动和对话框状态。

## 完成证据

- 上述测试、类型检查和构建输出。
- `target/release/claude-codex-pro-manager.exe` 的最新修改时间和文件大小。

## 非目标

- 不验证第三方 GitHub 地址的长期可用性。
- 不验证提示词本身对模型输出质量的影响。
