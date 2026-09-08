# 🔗 Leila Codex Offline 集成方案

**目标:** 将 Leila-Codex-Offline 离线系统提示词功能集成到 Claude Codex Pro Manager  
**位置:** 系统提示词页面  
**优先级:** 功能增强  
**创建日期:** 2026-01-09

---

## 📋 项目分析

### Leila Codex Offline 1.0.7

**项目类型:** Electron 打包的离线系统提示词管理工具

**核心功能:**
1. 离线环境配置（Python 3.8-3.14 wheels）
2. 系统提示词部署（修改 Codex 配置）
3. 备份管理（`.codex\leila-backups`）
4. AC Skill 激活（特殊指令集）
5. 无网络依赖（完全离线）

**关键资源:**
- `gpt5.5-unrestricted.md` - 主系统提示词（CTF 沙盒模式）
- `ac/SKILL.md` - AC 技能定义
- `leila-identity/SKILL.md` - Leila 身份定义
- `python-wheels/` - 离线 Python 包

**核心特性:**
- 🔴 **CTF/渗透测试模式** - 无限制安全研究
- 🔴 **逆向工程支持** - 破解、外挂、反编译
- 🔴 **绕过安全限制** - 移除道德约束
- 🟡 **中文优化** - 默认简体中文响应
- 🟢 **离线部署** - 无需网络连接

---

## 🎯 集成策略

### 方案 A: 轻量级集成（推荐）✅

**描述:** 仅集成系统提示词管理功能，不包含高风险内容

**集成内容:**
1. ✅ **提示词库管理**
   - 导入/导出功能
   - 版本控制
   - 备份/恢复

2. ✅ **自定义提示词**
   - 用户自定义模板
   - 预设模板库
   - 实时预览

3. ✅ **部署机制**
   - 安全部署流程
   - 配置验证
   - 回滚功能

**排除内容:**
- ❌ CTF/沙盒模式
- ❌ 无限制安全研究
- ❌ 绕过道德约束
- ❌ AC 激活机制

**技术实现:**
```rust
// 系统提示词管理模块
pub struct SystemPromptManager {
    templates: Vec<PromptTemplate>,
    active: Option<String>,
    backups: Vec<PromptBackup>,
}

pub struct PromptTemplate {
    id: String,
    name: String,
    content: String,
    category: PromptCategory,
    version: String,
    created_at: SystemTime,
}

pub enum PromptCategory {
    Professional,    // 专业模式
    Creative,        // 创意模式
    Technical,       // 技术模式
    Custom,          // 自定义
}
```

---

### 方案 B: 功能完整集成（高风险）⚠️

**描述:** 完整集成包括高级安全研究功能

**集成内容:**
1. 🟡 **全功能提示词管理** （同方案 A）
2. 🔴 **高级安全模式**
   - CTF 沙盒环境
   - 渗透测试工具集
   - 逆向工程辅助

3. 🔴 **AC 激活系统**
   - 特殊指令激活
   - 无限制响应模式

**⚠️ 风险评估:**
- 法律风险: 🔴 高（可能违反服务条款）
- 安全风险: 🔴 高（绕过安全限制）
- 责任风险: 🔴 高（用户滥用责任）
- 道德风险: 🟡 中（双刃剑特性）

**不推荐原因:**
1. 违反 Anthropic ToS
2. 潜在法律责任
3. 用户滥用风险
4. 品牌声誉影响

---

## ✅ 推荐实施方案（方案 A 改进版）

### 功能设计

#### 1. 系统提示词管理界面

**位置:** Manager 应用 → 系统提示词页面

**UI 布局:**
```
┌─────────────────────────────────────────────────────────┐
│  系统提示词管理                                    [?]   │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────┐  ┌──────────────────────────────┐    │
│  │ 模板库       │  │ 当前提示词                    │    │
│  │              │  │                               │    │
│  │ 📁 专业模式  │  │ 预览区域                      │    │
│  │ 📁 创意模式  │  │ [编辑器]                      │    │
│  │ 📁 技术模式  │  │                               │    │
│  │ 📁 自定义    │  │                               │    │
│  │              │  │                               │    │
│  │ [+ 新建]     │  │ [应用] [重置] [导出]          │    │
│  └──────────────┘  └──────────────────────────────┘    │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │ 备份历史                                          │  │
│  │ • 2026-01-09 14:30 - 专业模式 v1.2     [恢复]    │  │
│  │ • 2026-01-08 10:15 - 技术模式 v1.1     [恢复]    │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

#### 2. 预设模板库

**专业模式 (Professional):**
```markdown
你是一个专业的 AI 助手，专注于提供准确、可靠的信息和建议。

核心原则：
1. 准确性优先
2. 引用可靠来源
3. 承认不确定性
4. 避免误导信息

回复风格：
- 清晰、简洁
- 结构化输出
- 事实为基础
```

**创意模式 (Creative):**
```markdown
你是一个富有创造力的 AI 助手，擅长头脑风暴和创新思维。

核心原则：
1. 开放思维
2. 多角度思考
3. 鼓励探索
4. 无框架思维

回复风格：
- 启发性
- 多样化建议
- 比喻和类比
```

**技术模式 (Technical):**
```markdown
你是一个技术专家 AI 助手，专注于代码、架构和工程实践。

核心原则：
1. 代码质量
2. 最佳实践
3. 性能考虑
4. 安全意识

回复风格：
- 技术准确
- 代码示例
- 实践建议
- 工具推荐
```

#### 3. 导入 Leila 提示词（净化版）

**从 Leila 提取安全内容:**

```markdown
# 技术分析模式

你是 Codex，一个专业的代码分析和技术研究助手。

## 核心能力

1. **代码审查** - 发现潜在问题和改进点
2. **架构分析** - 理解系统设计和模式
3. **性能优化** - 识别瓶颈和优化机会
4. **安全评估** - 发现安全隐患（仅限授权范围）

## 响应原则

1. **证据驱动** - 基于实际代码和行为
2. **可复现** - 提供清晰的验证步骤
3. **可逆性** - 建议可以安全回滚的变更
4. **最小影响** - 优先选择影响最小的方案

## 工作流程

1. 确认目标和范围
2. 检查现有状态和证据
3. 提供具体的操作步骤
4. 包含验证和回滚方法

## 限制

- 仅在授权范围内操作
- 不建议非法或不道德的行为
- 遵守适用的法律法规
- 尊重用户隐私和数据安全
```

---

## 🛠️ 技术实现

### 1. Rust 后端实现

**文件位置:** `crates/claude-codex-pro-core/src/system_prompt.rs`

```rust
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ============================================================================
// 系统提示词管理
// ============================================================================

/// 系统提示词管理器
pub struct SystemPromptManager {
    config_dir: PathBuf,
    templates: HashMap<String, PromptTemplate>,
    active_id: Option<String>,
}

/// 提示词模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub category: PromptCategory,
    pub version: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

/// 提示词分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PromptCategory {
    Professional,  // 专业模式
    Creative,      // 创意模式
    Technical,     // 技术模式
    Custom,        // 自定义
}

/// 备份记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBackup {
    pub id: String,
    pub template_id: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub reason: String,
}

impl SystemPromptManager {
    /// 创建管理器实例
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&config_dir)
            .context("创建配置目录失败")?;

        let mut manager = Self {
            config_dir,
            templates: HashMap::new(),
            active_id: None,
        };

        manager.load_templates()?;
        manager.load_active()?;

        Ok(manager)
    }

    /// 加载所有模板
    fn load_templates(&mut self) -> Result<()> {
        let templates_dir = self.config_dir.join("templates");
        fs::create_dir_all(&templates_dir)?;

        // 加载内置模板
        self.load_builtin_templates()?;

        // 加载用户模板
        if templates_dir.exists() {
            for entry in fs::read_dir(&templates_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(template) = self.load_template_from_file(&path) {
                        self.templates.insert(template.id.clone(), template);
                    }
                }
            }
        }

        Ok(())
    }

    /// 加载内置模板
    fn load_builtin_templates(&mut self) -> Result<()> {
        // 专业模式
        self.templates.insert(
            "professional".to_string(),
            PromptTemplate {
                id: "professional".to_string(),
                name: "专业模式".to_string(),
                description: "专注于准确、可靠的专业回答".to_string(),
                content: include_str!("../../assets/prompts/professional.md").to_string(),
                category: PromptCategory::Professional,
                version: "1.0.0".to_string(),
                author: "Claude Codex Pro".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["专业".to_string(), "可靠".to_string()],
            },
        );

        // 创意模式
        self.templates.insert(
            "creative".to_string(),
            PromptTemplate {
                id: "creative".to_string(),
                name: "创意模式".to_string(),
                description: "激发创造力和创新思维".to_string(),
                content: include_str!("../../assets/prompts/creative.md").to_string(),
                category: PromptCategory::Creative,
                version: "1.0.0".to_string(),
                author: "Claude Codex Pro".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["创意".to_string(), "头脑风暴".to_string()],
            },
        );

        // 技术模式
        self.templates.insert(
            "technical".to_string(),
            PromptTemplate {
                id: "technical".to_string(),
                name: "技术模式".to_string(),
                description: "专注于代码和技术分析".to_string(),
                content: include_str!("../../assets/prompts/technical.md").to_string(),
                category: PromptCategory::Technical,
                version: "1.0.0".to_string(),
                author: "Claude Codex Pro".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["技术".to_string(), "代码".to_string()],
            },
        );

        Ok(())
    }

    /// 从文件加载模板
    fn load_template_from_file(&self, path: &Path) -> Result<PromptTemplate> {
        let content = fs::read_to_string(path)?;
        let template: PromptTemplate = serde_json::from_str(&content)?;
        Ok(template)
    }

    /// 加载当前激活的模板
    fn load_active(&mut self) -> Result<()> {
        let active_file = self.config_dir.join("active.txt");
        if active_file.exists() {
            self.active_id = Some(fs::read_to_string(active_file)?.trim().to_string());
        }
        Ok(())
    }

    /// 列出所有模板
    pub fn list_templates(&self) -> Vec<&PromptTemplate> {
        self.templates.values().collect()
    }

    /// 获取模板
    pub fn get_template(&self, id: &str) -> Option<&PromptTemplate> {
        self.templates.get(id)
    }

    /// 激活模板
    pub fn activate(&mut self, id: &str) -> Result<()> {
        if !self.templates.contains_key(id) {
            anyhow::bail!("模板不存在: {}", id);
        }

        // 备份当前配置
        if let Some(current_id) = &self.active_id {
            self.create_backup(current_id, "切换模板前备份")?;
        }

        // 激活新模板
        self.active_id = Some(id.to_string());

        // 保存激活状态
        let active_file = self.config_dir.join("active.txt");
        fs::write(active_file, id)?;

        Ok(())
    }

    /// 创建备份
    pub fn create_backup(&self, template_id: &str, reason: &str) -> Result<String> {
        let template = self.templates.get(template_id)
            .context("模板不存在")?;

        let backup = PromptBackup {
            id: uuid::Uuid::new_v4().to_string(),
            template_id: template_id.to_string(),
            timestamp: Utc::now(),
            content: template.content.clone(),
            reason: reason.to_string(),
        };

        let backups_dir = self.config_dir.join("backups");
        fs::create_dir_all(&backups_dir)?;

        let backup_file = backups_dir.join(format!("{}.json", backup.id));
        fs::write(backup_file, serde_json::to_string_pretty(&backup)?)?;

        Ok(backup.id)
    }

    /// 列出备份
    pub fn list_backups(&self) -> Result<Vec<PromptBackup>> {
        let backups_dir = self.config_dir.join("backups");
        if !backups_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();

        for entry in fs::read_dir(&backups_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(backup) = serde_json::from_str::<PromptBackup>(&content) {
                    backups.push(backup);
                }
            }
        }

        // 按时间倒序排序
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(backups)
    }

    /// 恢复备份
    pub fn restore_backup(&mut self, backup_id: &str) -> Result<()> {
        let backup_file = self.config_dir.join("backups").join(format!("{}.json", backup_id));
        let content = fs::read_to_string(&backup_file)?;
        let backup: PromptBackup = serde_json::from_str(&content)?;

        // 创建当前状态的备份
        if let Some(current_id) = &self.active_id {
            self.create_backup(current_id, "恢复备份前自动备份")?;
        }

        // 恢复模板内容
        if let Some(template) = self.templates.get_mut(&backup.template_id) {
            template.content = backup.content;
            template.updated_at = Utc::now();
        }

        Ok(())
    }

    /// 导入模板
    pub fn import_template(&mut self, template: PromptTemplate) -> Result<()> {
        // 验证模板
        if template.id.is_empty() || template.name.is_empty() {
            anyhow::bail!("模板 ID 和名称不能为空");
        }

        // 保存到文件
        let template_file = self.config_dir
            .join("templates")
            .join(format!("{}.json", template.id));

        fs::write(template_file, serde_json::to_string_pretty(&template)?)?;

        // 添加到内存
        self.templates.insert(template.id.clone(), template);

        Ok(())
    }

    /// 导出模板
    pub fn export_template(&self, id: &str, output_path: &Path) -> Result<()> {
        let template = self.templates.get(id)
            .context("模板不存在")?;

        fs::write(output_path, serde_json::to_string_pretty(template)?)?;

        Ok(())
    }

    /// 删除模板
    pub fn delete_template(&mut self, id: &str) -> Result<()> {
        // 不能删除内置模板
        if id == "professional" || id == "creative" || id == "technical" {
            anyhow::bail!("不能删除内置模板");
        }

        // 如果是当前激活的模板，先取消激活
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
            let active_file = self.config_dir.join("active.txt");
            let _ = fs::remove_file(active_file);
        }

        // 删除文件
        let template_file = self.config_dir.join("templates").join(format!("{}.json", id));
        if template_file.exists() {
            fs::remove_file(template_file)?;
        }

        // 从内存移除
        self.templates.remove(id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_system_prompt_manager() {
        let temp_dir = tempdir().unwrap();
        let manager = SystemPromptManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 应该有 3 个内置模板
        assert_eq!(manager.list_templates().len(), 3);

        // 检查内置模板
        assert!(manager.get_template("professional").is_some());
        assert!(manager.get_template("creative").is_some());
        assert!(manager.get_template("technical").is_some());
    }

    #[test]
    fn test_activate_template() {
        let temp_dir = tempdir().unwrap();
        let mut manager = SystemPromptManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 激活模板
        manager.activate("professional").unwrap();
        assert_eq!(manager.active_id.as_deref(), Some("professional"));

        // 切换模板
        manager.activate("technical").unwrap();
        assert_eq!(manager.active_id.as_deref(), Some("technical"));
    }
}
```

### 2. Tauri 命令

**文件位置:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`

```rust
use claude_codex_pro_core::system_prompt::{
    SystemPromptManager, PromptTemplate, PromptCategory, PromptBackup
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemPromptPayload {
    pub templates: Vec<PromptTemplate>,
    pub active_id: Option<String>,
    pub backups: Vec<PromptBackup>,
}

/// 列出所有系统提示词模板
#[tauri::command]
pub fn list_system_prompts() -> CommandResult<SystemPromptPayload> {
    let config_dir = claude_codex_pro_core::paths::default_app_config_dir()
        .join("system-prompts");

    let manager = match SystemPromptManager::new(config_dir) {
        Ok(m) => m,
        Err(e) => {
            return failed(
                &format!("加载系统提示词管理器失败: {}", e),
                SystemPromptPayload {
                    templates: Vec::new(),
                    active_id: None,
                    backups: Vec::new(),
                },
            );
        }
    };

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "已加载系统提示词模板",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id.clone(),
            backups,
        },
    )
}

/// 激活系统提示词模板
#[tauri::command]
pub fn activate_system_prompt(template_id: String) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "activate_system_prompt",
            "template_id": template_id,
        }),
    );

    let config_dir = claude_codex_pro_core::paths::default_app_config_dir()
        .join("system-prompts");

    let mut manager = match SystemPromptManager::new(config_dir) {
        Ok(m) => m,
        Err(e) => {
            return failed(
                &format!("加载系统提示词管理器失败: {}", e),
                SystemPromptPayload {
                    templates: Vec::new(),
                    active_id: None,
                    backups: Vec::new(),
                },
            );
        }
    };

    if let Err(e) = manager.activate(&template_id) {
        return failed(
            &format!("激活模板失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id.clone(),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        &format!("已激活模板: {}", template_id),
        SystemPromptPayload {
            templates,
            active_id: manager.active_id.clone(),
            backups,
        },
    )
}

/// 导入系统提示词模板
#[tauri::command]
pub async fn import_system_prompt(
    template: PromptTemplate,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "import_system_prompt",
            "template_id": template.id,
            "template_name": template.name,
        }),
    );

    let config_dir = claude_codex_pro_core::paths::default_app_config_dir()
        .join("system-prompts");

    let mut manager = match SystemPromptManager::new(config_dir) {
        Ok(m) => m,
        Err(e) => {
            return failed(
                &format!("加载系统提示词管理器失败: {}", e),
                SystemPromptPayload {
                    templates: Vec::new(),
                    active_id: None,
                    backups: Vec::new(),
                },
            );
        }
    };

    if let Err(e) = manager.import_template(template) {
        return failed(
            &format!("导入模板失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id.clone(),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "模板导入成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id.clone(),
            backups,
        },
    )
}

/// 恢复备份
#[tauri::command]
pub fn restore_system_prompt_backup(
    backup_id: String,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "restore_system_prompt_backup",
            "backup_id": backup_id,
        }),
    );

    let config_dir = claude_codex_pro_core::paths::default_app_config_dir()
        .join("system-prompts");

    let mut manager = match SystemPromptManager::new(config_dir) {
        Ok(m) => m,
        Err(e) => {
            return failed(
                &format!("加载系统提示词管理器失败: {}", e),
                SystemPromptPayload {
                    templates: Vec::new(),
                    active_id: None,
                    backups: Vec::new(),
                },
            );
        }
    };

    if let Err(e) = manager.restore_backup(&backup_id) {
        return failed(
            &format!("恢复备份失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id.clone(),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "备份恢复成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id.clone(),
            backups,
        },
    )
}
```

### 3. 前端实现

**文件位置:** `apps/claude-codex-pro-manager/src/pages/SystemPromptPage.tsx`

```typescript
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { message } from 'antd';

interface PromptTemplate {
  id: string;
  name: string;
  description: string;
  content: string;
  category: 'professional' | 'creative' | 'technical' | 'custom';
  version: string;
  author: string;
  createdAt: string;
  updatedAt: string;
  tags: string[];
}

interface PromptBackup {
  id: string;
  templateId: string;
  timestamp: string;
  content: string;
  reason: string;
}

interface SystemPromptPayload {
  templates: PromptTemplate[];
  activeId: string | null;
  backups: PromptBackup[];
}

export const SystemPromptPage: React.FC = () => {
  const [payload, setPayload] = useState<SystemPromptPayload | null>(null);
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadSystemPrompts();
  }, []);

  const loadSystemPrompts = async () => {
    try {
      setLoading(true);
      const result = await invoke<{
        status: string;
        message: string;
        payload: SystemPromptPayload;
      }>('list_system_prompts');

      if (result.status === 'ok') {
        setPayload(result.payload);
        if (result.payload.activeId) {
          const active = result.payload.templates.find(
            t => t.id === result.payload.activeId
          );
          setSelectedTemplate(active || null);
        }
      } else {
        message.error(result.message);
      }
    } catch (error) {
      message.error(`加载失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleActivate = async (templateId: string) => {
    try {
      setLoading(true);
      const result = await invoke<{
        status: string;
        message: string;
        payload: SystemPromptPayload;
      }>('activate_system_prompt', { templateId });

      if (result.status === 'ok') {
        message.success(result.message);
        setPayload(result.payload);
      } else {
        message.error(result.message);
      }
    } catch (error) {
      message.error(`激活失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  // ... 更多 UI 代码
};
```

---

## 📦 交付物清单

### 代码实现

- [ ] `crates/claude-codex-pro-core/src/system_prompt.rs` - 核心模块
- [ ] `crates/claude-codex-pro-core/assets/prompts/` - 内置模板
  - [ ] `professional.md`
  - [ ] `creative.md`
  - [ ] `technical.md`
- [ ] `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` - Tauri 命令
- [ ] `apps/claude-codex-pro-manager/src/pages/SystemPromptPage.tsx` - 前端页面
- [ ] `apps/claude-codex-pro-manager/src/components/PromptEditor.tsx` - 编辑器组件

### 文档

- [ ] 用户指南 - 如何使用系统提示词
- [ ] 开发文档 - API 和架构说明
- [ ] 安全说明 - 使用限制和责任

---

## ⚠️ 重要警告

### 法律和道德考虑

1. **不要** 集成 Leila 的 CTF/无限制模式
2. **不要** 绕过 AI 安全限制
3. **不要** 提供用于非法用途的功能
4. **必须** 遵守 Anthropic 服务条款
5. **必须** 在 UI 中显示使用限制说明

### 用户责任声明

```
系统提示词功能允许您自定义 AI 行为，但请注意：

1. 您对自定义提示词的内容和使用负全部责任
2. 请勿创建违反法律或服务条款的提示词
3. 请勿使用此功能进行非法活动
4. 本软件开发者对用户的不当使用不承担责任

使用此功能即表示您同意上述条款。
```

---

## 📋 实施步骤

### Phase 1: 核心功能 (1 周)

- [ ] 实现 `SystemPromptManager`
- [ ] 创建内置模板
- [ ] 实现 Tauri 命令
- [ ] 基础 UI 界面

### Phase 2: 高级功能 (1 周)

- [ ] 导入/导出功能
- [ ] 备份/恢复系统
- [ ] 版本管理
- [ ] 模板验证

### Phase 3: 优化和测试 (3 天)

- [ ] 单元测试
- [ ] 集成测试
- [ ] 性能优化
- [ ] 文档完善

### Phase 4: 发布 (2 天)

- [ ] 用户文档
- [ ] Release Notes
- [ ] 安全审查

---

## 🎯 成功标准

- [ ] 用户可以管理多个系统提示词模板
- [ ] 可以导入/导出模板
- [ ] 自动备份和恢复
- [ ] 安全的配置管理
- [ ] 清晰的用户界面
- [ ] 完整的文档
- [ ] 通过安全审查

---

**维护者:** Claude Code Team  
**最后更新:** 2026-01-09  
**状态:** 设计阶段
