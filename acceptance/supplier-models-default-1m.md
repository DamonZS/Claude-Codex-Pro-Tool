# 供应商模型默认 1M 上下文验收

对应规格：`spec/supplier-models-default-1m.md`。

## 通过标准

1. TypeScript 类型检查通过。
2. 供应商模型转换函数对缺失 Codex 上下文窗口默认勾选 1M，序列化目录输出 `contextWindow: 1000000`。
3. Claude 供应商路径的模型映射和 1M 标记行为不发生变化。
4. Codex 原生目录读取缺少上下文窗口时，模型描述符使用 `1000000`。
5. 保存 Codex 供应商后，勾选 1M 的模型生成 `context_window` 与 `max_context_window`；取消勾选的模型不生成 `context_window`，但保留 `max_context_window`；无映射时也由主模型/模型列表生成目录。
6. 与内置目录 slug 匹配的模型保留其原生 `max_context_window`，未知上游模型默认最大值为 `1000000`。
7. Codex 模型列表 UI 显示“1M”复选框，新增模型和获取模型默认勾选，取消勾选可持久化为 `supports1m: false`。

## 验证方式

- `npm --prefix apps/claude-codex-pro-manager run check`
- `npm --prefix apps/claude-codex-pro-manager run vite:build`
- 对 `src/lib/supplier.ts` 的默认值路径做定向源码检查。
- 运行 `cargo test -p claude-codex-pro-core --test relay_config apply_relay_profile_maps_codex_catalog_to_native_model_catalog_json -- --exact`，断言生成的目录上下文字段及无映射路径。

## 非目标

- 不要求调用真实供应商 `/models` 接口。
- 不修改现有未提交的其他功能改动。
