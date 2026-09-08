# 🔧 修复智能体通讯 call_id 问题

**问题:** `function_call_output requires call_id on HTTP requests`

**状态:** 🔴 阻塞性bug - 智能体无法正常通讯

---

## 📋 问题描述

### 症状

1. ✅ 智能体通讯被发送到用户会话窗口
2. ❌ 会话窗口报错：`function_call_output requires call_id on HTTP requests`
3. ❌ 无法继续对话

### 错误截图分析

根据用户提供的截图：
- 智能体通讯消息出现在会话中
- 包含错误信息：`function_call_output requires call_id on HTTP requests; continuation via previous_response_id is only supported on Responses WebSocket v2`
- 会话被阻塞

---

## 🔍 根本原因分析

### 已有的修复机制

**文件:** `crates/claude-codex-pro-core/src/protocol_proxy.rs`

```rust
// 第971行 - 应该自动补全 call_id
normalize_responses_tool_output_call_ids(&mut request_json);
```

**函数实现:**  (第436-477行)
```rust
fn normalize_responses_tool_output_call_ids(body: &mut Value) {
    // ... 自动补全 call_id 的逻辑
}
```

### 为什么没有生效？

**可能的原因:**

1. **智能体通讯走了不同的代码路径**
   - 可能通过 WebSocket 发送
   - 可能通过内部消息传递
   - 绕过了 normalize 函数

2. **call_id 匹配失败**
   - `known_call_ids` 集合没有包含正确的 ID
   - 工具调用和工具输出的 ID 不匹配

3. **时序问题**
   - 工具输出先于工具调用到达
   - normalize 时还没有 `known_call_ids`

---

## 🔬 诊断步骤

### 步骤 1: 检查智能体通讯路径

<function_calls>
<invoke name="execute_bash">
<parameter name="command">cd D:/Project/Claude-Codex-Pro-Tool && git commit -am "📊 [DOCS] 添加智能体 call_id 问题诊断文档" && git push origin main
### 步骤 1: 启用详细日志

添加日志追踪智能体通讯。

### 步骤 2: 检查智能体消息路由

查找智能体通讯发送路径。

---

## 🔧 修复方案

### 方案 A: 强制 normalize（推荐）

在所有消息路由添加 normalize 调用。

### 方案 B: 构建时添加 call_id

在工具输出构建时直接携带 call_id。

### 方案 C: 前端过滤（临时）

增强前端错误消息过滤。

---

## 🎯 推荐实施

1. **临时:** 前端过滤
2. **根本:** 添加 normalize
3. **长期:** 统一消息路由

---

**状态:** 🔴 Critical - P0  
**日期:** 2026-01-09
