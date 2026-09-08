# 🚀 系统提示词功能完整实施指南

**版本:** 1.0.0  
**创建日期:** 2026-01-09  
**状态:** 实施中

---

## 📋 目录

1. [功能概述](#功能概述)
2. [已完成部分](#已完成部分)
3. [待实施部分](#待实施部分)
4. [详细实施步骤](#详细实施步骤)
5. [测试指南](#测试指南)
6. [发布清单](#发布清单)

---

## 📦 功能概述

### 核心功能

**系统提示词管理器** 允许用户：
1. ✅ 管理多个提示词模板
2. ✅ 切换不同的AI行为模式
3. ✅ 导入/导出自定义模板
4. ✅ 自动备份和恢复
5. ✅ 按分类浏览模板

### 内置模板

1. **专业模式** - 准确、可靠的专业回答
2. **创意模式** - 激发创造力和创新思维  
3. **技术模式** - 代码和技术分析专家
4. **研究模式** - 深入分析和系统性研究

---

## ✅ 已完成部分

### 1. Rust 核心模块 ✅

**文件:** `crates/claude-codex-pro-core/src/system_prompt.rs`

**已实现功能:**
- [x] `SystemPromptManager` 核心结构
- [x] 模板 CRUD 操作
- [x] 激活/取消激活
- [x] 备份/恢复系统
- [x] 导入/导出功能
- [x] 按分类筛选
- [x] 自动备份清理
- [x] 4 个内置模板
- [x] 单元测试

**关键 API:**
```rust
// 创建管理器
let manager = SystemPromptManager::new(config_dir)?;

// 列出模板
let templates = manager.list_templates();

// 激活模板
manager.activate("professional")?;

// 创建备份
let backup_id = manager.create_backup("professional", "手动备份")?;

// 恢复备份
manager.restore_backup(&backup_id)?;
```

---

## 📝 待实施部分

### 2. Tauri 命令层 ⏳

**文件:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`

**需要添加的命令:**

```rust
// ============================================================================
// 系统提示词管理命令
// ============================================================================

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
            active_id: manager.active_id().map(|s| s.to_string()),
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
                active_id: manager.active_id().map(|s| s.to_string()),
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
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}

/// 取消激活系统提示词
#[tauri::command]
pub fn deactivate_system_prompt() -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({"operation": "deactivate_system_prompt"}),
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

    if let Err(e) = manager.deactivate() {
        return failed(
            &format!("取消激活失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id().map(|s| s.to_string()),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "已取消激活系统提示词",
        SystemPromptPayload {
            templates,
            active_id: None,
            backups,
        },
    )
}

/// 创建新模板
#[tauri::command]
pub fn create_system_prompt_template(
    template: PromptTemplate,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "create_system_prompt_template",
            "template_id": template.id,
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

    if let Err(e) = manager.create_template(template) {
        return failed(
            &format!("创建模板失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id().map(|s| s.to_string()),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "模板创建成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}

/// 更新模板内容
#[tauri::command]
pub fn update_system_prompt_template(
    template_id: String,
    content: String,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "update_system_prompt_template",
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

    if let Err(e) = manager.update_template(&template_id, content) {
        return failed(
            &format!("更新模板失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id().map(|s| s.to_string()),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "模板更新成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}

/// 删除模板
#[tauri::command]
pub fn delete_system_prompt_template(
    template_id: String,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "delete_system_prompt_template",
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

    if let Err(e) = manager.delete_template(&template_id) {
        return failed(
            &format!("删除模板失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id().map(|s| s.to_string()),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "模板删除成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}

/// 导入模板
#[tauri::command]
pub async fn import_system_prompt_template(
    template: PromptTemplate,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "import_system_prompt_template",
            "template_id": template.id,
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
                active_id: manager.active_id().map(|s| s.to_string()),
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
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}

/// 导出模板
#[tauri::command]
pub async fn export_system_prompt_template(
    template_id: String,
    output_path: String,
) -> CommandResult<()> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "export_system_prompt_template",
            "template_id": template_id,
        }),
    );

    let config_dir = claude_codex_pro_core::paths::default_app_config_dir()
        .join("system-prompts");

    let manager = match SystemPromptManager::new(config_dir) {
        Ok(m) => m,
        Err(e) => {
            return failed(&format!("加载系统提示词管理器失败: {}", e), ());
        }
    };

    let output_path = PathBuf::from(output_path);
    if let Err(e) = manager.export_template(&template_id, &output_path) {
        return failed(&format!("导出模板失败: {}", e), ());
    }

    ok("模板导出成功", ())
}

/// 创建备份
#[tauri::command]
pub fn create_system_prompt_backup(
    template_id: String,
    reason: String,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "create_system_prompt_backup",
            "template_id": template_id,
        }),
    );

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

    if let Err(e) = manager.create_backup(&template_id, &reason) {
        return failed(
            &format!("创建备份失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id().map(|s| s.to_string()),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "备份创建成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id().map(|s| s.to_string()),
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
                active_id: manager.active_id().map(|s| s.to_string()),
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
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}

/// 删除备份
#[tauri::command]
pub fn delete_system_prompt_backup(
    backup_id: String,
) -> CommandResult<SystemPromptPayload> {
    log_security_event(
        SecurityEventType::ConfigurationChange,
        json!({
            "operation": "delete_system_prompt_backup",
            "backup_id": backup_id,
        }),
    );

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

    if let Err(e) = manager.delete_backup(&backup_id) {
        return failed(
            &format!("删除备份失败: {}", e),
            SystemPromptPayload {
                templates: manager.list_templates().into_iter().cloned().collect(),
                active_id: manager.active_id().map(|s| s.to_string()),
                backups: manager.list_backups().unwrap_or_default(),
            },
        );
    }

    let templates = manager.list_templates().into_iter().cloned().collect();
    let backups = manager.list_backups().unwrap_or_default();

    ok(
        "备份删除成功",
        SystemPromptPayload {
            templates,
            active_id: manager.active_id().map(|s| s.to_string()),
            backups,
        },
    )
}
```

**注册命令:**
在 `main.rs` 的 `tauri::Builder` 中添加：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令 ...
    
    // 系统提示词命令
    list_system_prompts,
    activate_system_prompt,
    deactivate_system_prompt,
    create_system_prompt_template,
    update_system_prompt_template,
    delete_system_prompt_template,
    import_system_prompt_template,
    export_system_prompt_template,
    create_system_prompt_backup,
    restore_system_prompt_backup,
    delete_system_prompt_backup,
])
```

---

### 3. 前端 TypeScript 类型 ⏳

**文件:** `apps/claude-codex-pro-manager/src/types/system-prompt.ts`

```typescript
export interface PromptTemplate {
  id: string;
  name: string;
  description: string;
  content: string;
  category: PromptCategory;
  version: string;
  author: string;
  createdAt: string;
  updatedAt: string;
  tags: string[];
}

export type PromptCategory = 
  | 'professional' 
  | 'creative' 
  | 'technical' 
  | 'research' 
  | 'custom';

export interface PromptBackup {
  id: string;
  templateId: string;
  timestamp: string;
  content: string;
  reason: string;
}

export interface SystemPromptPayload {
  templates: PromptTemplate[];
  activeId: string | null;
  backups: PromptBackup[];
}

export interface CommandResult<T> {
  status: 'ok' | 'failed';
  message: string;
  payload: T;
}
```

---

### 4. 前端 API 服务 ⏳

**文件:** `apps/claude-codex-pro-manager/src/services/system-prompt.service.ts`

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import type { 
  PromptTemplate, 
  PromptBackup, 
  SystemPromptPayload, 
  CommandResult 
} from '../types/system-prompt';

export class SystemPromptService {
  /**
   * 列出所有系统提示词模板
   */
  static async listPrompts(): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'list_system_prompts'
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 激活系统提示词模板
   */
  static async activatePrompt(templateId: string): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'activate_system_prompt',
      { templateId }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 取消激活
   */
  static async deactivatePrompt(): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'deactivate_system_prompt'
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 创建新模板
   */
  static async createTemplate(template: PromptTemplate): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'create_system_prompt_template',
      { template }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 更新模板
   */
  static async updateTemplate(
    templateId: string, 
    content: string
  ): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'update_system_prompt_template',
      { templateId, content }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 删除模板
   */
  static async deleteTemplate(templateId: string): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'delete_system_prompt_template',
      { templateId }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 导入模板
   */
  static async importTemplate(template: PromptTemplate): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'import_system_prompt_template',
      { template }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 导出模板
   */
  static async exportTemplate(templateId: string, outputPath: string): Promise<void> {
    const result = await invoke<CommandResult<void>>(
      'export_system_prompt_template',
      { templateId, outputPath }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
  }

  /**
   * 创建备份
   */
  static async createBackup(
    templateId: string, 
    reason: string
  ): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'create_system_prompt_backup',
      { templateId, reason }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 恢复备份
   */
  static async restoreBackup(backupId: string): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'restore_system_prompt_backup',
      { backupId }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }

  /**
   * 删除备份
   */
  static async deleteBackup(backupId: string): Promise<SystemPromptPayload> {
    const result = await invoke<CommandResult<SystemPromptPayload>>(
      'delete_system_prompt_backup',
      { backupId }
    );
    
    if (result.status !== 'ok') {
      throw new Error(result.message);
    }
    
    return result.payload;
  }
}
```

---

### 5. 前端页面组件 ⏳

**文件:** `apps/claude-codex-pro-manager/src/pages/SystemPromptPage.tsx`

由于内容较长，请查看完整实现代码（见下一部分）。

---

## 🔨 详细实施步骤

### Step 1: 编译核心模块

```bash
cd D:/Project/Claude-Codex-Pro-Tool
cargo build --package claude-codex-pro-core
cargo test --package claude-codex-pro-core system_prompt
```

### Step 2: 添加 Tauri 命令

1. 复制上述 Tauri 命令代码到 `commands.rs`
2. 在 `main.rs` 中注册命令
3. 编译测试

```bash
cargo build --package claude-codex-pro-manager
```

### Step 3: 创建前端类型和服务

1. 创建类型定义文件
2. 创建服务类
3. 测试 API 调用

### Step 4: 实现前端页面

1. 创建主页面组件
2. 创建子组件（编辑器、列表、备份）
3. 集成路由

### Step 5: 测试

1. 单元测试
2. 集成测试
3. 手动测试

---

## 🧪 测试指南

### Rust 单元测试

```bash
# 测试核心模块
cargo test --package claude-codex-pro-core system_prompt -- --nocapture

# 测试特定功能
cargo test --package claude-codex-pro-core test_activate_template
cargo test --package claude-codex-pro-core test_backup_and_restore
```

### 前端测试

```bash
# 在 Manager 目录
cd apps/claude-codex-pro-manager
npm test
```

### 手动测试清单

- [ ] 列出模板
- [ ] 激活模板
- [ ] 切换模板
- [ ] 创建自定义模板
- [ ] 编辑模板内容
- [ ] 删除模板
- [ ] 导入模板文件
- [ ] 导出模板文件
- [ ] 创建备份
- [ ] 恢复备份
- [ ] 删除备份
- [ ] 验证持久化

---

## 📋 发布清单

### 代码完成度

- [x] Rust 核心模块
- [ ] Tauri 命令层
- [ ] 前端类型定义
- [ ] 前端服务层
- [ ] 前端UI组件
- [ ] 路由集成

### 测试

- [x] Rust 单元测试
- [ ] Tauri 命令测试
- [ ] 前端单元测试
- [ ] 集成测试
- [ ] 手动测试

### 文档

- [x] 功能设计文档
- [x] 实施指南
- [ ] 用户手册
- [ ] API 文档
- [ ] 示例和教程

### 发布准备

- [ ] CHANGELOG 更新
- [ ] Release Notes
- [ ] 版本号更新
- [ ] 编译检查
- [ ] 性能测试

---

## 📊 进度追踪

**总体进度:** 约 30% 完成

- ✅ Rust 核心模块 (100%)
- ⏳ Tauri 命令层 (0%)
- ⏳ 前端实现 (0%)
- ⏳ 测试 (20%)
- ⏳ 文档 (50%)

**预计完成时间:** 2 周

---

## 📞 支持和反馈

如有问题或建议，请：
1. 查看本文档
2. 查看 [LEILA_CODEX_INTEGRATION.md](./LEILA_CODEX_INTEGRATION.md)
3. 提交 GitHub Issue

---

**维护者:** Claude Code Team  
**最后更新:** 2026-01-09  
**状态:** 实施中 (30%)
