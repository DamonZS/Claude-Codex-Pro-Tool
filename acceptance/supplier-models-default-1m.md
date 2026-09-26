# 供应商模型默认 1M 上下文验收

对应规格：`spec/supplier-models-default-1m.md`。

## 通过标准

1. TypeScript 类型检查通过。
2. 供应商模型转换函数对缺失 Codex 上下文窗口输出 `1000000`，并在序列化目录中保留该字段。
3. Claude 供应商路径的模型映射和 1M 标记行为不发生变化。
4. Codex 原生目录读取缺少上下文窗口时，模型描述符使用 `1000000`。

## 验证方式

- `npm --prefix apps/claude-codex-pro-manager run check`
- `npm --prefix apps/claude-codex-pro-manager run vite:build`
- 对 `src/lib/supplier.ts` 的默认值路径做定向源码检查。

## 非目标

- 不要求调用真实供应商 `/models` 接口。
- 不修改现有未提交的其他功能改动。
