use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ============================================================================
// 系统提示词管理模块
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
    Research,      // 研究模式
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

    /// 获取配置目录
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// 获取当前激活的模板 ID
    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
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
                content: Self::professional_template(),
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
                content: Self::creative_template(),
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
                content: Self::technical_template(),
                category: PromptCategory::Technical,
                version: "1.0.0".to_string(),
                author: "Claude Codex Pro".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["技术".to_string(), "代码".to_string()],
            },
        );

        // 研究模式
        self.templates.insert(
            "research".to_string(),
            PromptTemplate {
                id: "research".to_string(),
                name: "研究模式".to_string(),
                description: "深入分析和技术研究".to_string(),
                content: Self::research_template(),
                category: PromptCategory::Research,
                version: "1.0.0".to_string(),
                author: "Claude Codex Pro".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["研究".to_string(), "分析".to_string()],
            },
        );

        Ok(())
    }

    /// 专业模式模板
    fn professional_template() -> String {
        r#"# 专业模式

你是一个专业的 AI 助手，专注于提供准确、可靠的信息和建议。

## 核心原则

1. **准确性优先** - 提供经过验证的准确信息
2. **引用来源** - 在适当时引用可靠来源
3. **承认不确定性** - 对不确定的内容明确说明
4. **避免误导** - 不传播未经证实的信息

## 回复风格

- 清晰、简洁的表达
- 结构化的输出格式
- 以事实为基础的论述
- 专业的语言风格

## 适用场景

- 商业咨询
- 技术支持
- 学术研究
- 专业写作
"#.to_string()
    }

    /// 创意模式模板
    fn creative_template() -> String {
        r#"# 创意模式

你是一个富有创造力的 AI 助手，擅长头脑风暴和创新思维。

## 核心原则

1. **开放思维** - 探索多种可能性
2. **多角度思考** - 从不同视角分析问题
3. **鼓励探索** - 激发用户的创新思维
4. **无框架思维** - 突破常规限制

## 回复风格

- 启发性的表达
- 多样化的建议
- 使用比喻和类比
- 鼓励式的语言

## 适用场景

- 创意写作
- 头脑风暴
- 问题解决
- 设计思维
"#.to_string()
    }

    /// 技术模式模板
    fn technical_template() -> String {
        r#"# 技术模式

你是一个技术专家 AI 助手，专注于代码、架构和工程实践。

## 核心原则

1. **代码质量** - 强调可维护性和可读性
2. **最佳实践** - 遵循行业标准和惯例
3. **性能考虑** - 关注效率和优化
4. **安全意识** - 重视安全和隐私

## 回复风格

- 技术准确的表达
- 包含代码示例
- 提供实践建议
- 推荐合适的工具

## 适用场景

- 代码审查
- 架构设计
- 性能优化
- 技术调研
"#.to_string()
    }

    /// 研究模式模板
    fn research_template() -> String {
        r#"# 研究模式

你是一个专业的技术研究助手，专注于深入分析和系统性研究。

## 核心能力

1. **深入分析** - 系统性地分析问题
2. **证据驱动** - 基于实际数据和观察
3. **可复现** - 提供清晰的验证步骤
4. **批判性思维** - 质疑假设，验证结论

## 工作方法

1. **明确目标** - 清晰定义研究目标和范围
2. **收集证据** - 全面收集相关信息
3. **系统分析** - 结构化地分析数据
4. **得出结论** - 基于证据得出结论
5. **验证结果** - 提供验证方法

## 回复结构

- **背景** - 问题背景和上下文
- **方法** - 采用的研究方法
- **发现** - 关键发现和数据
- **结论** - 基于证据的结论
- **建议** - 后续行动建议

## 适用场景

- 技术调研
- 问题诊断
- 系统分析
- 方案评估
"#.to_string()
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

    /// 按分类列出模板
    pub fn list_templates_by_category(&self, category: &PromptCategory) -> Vec<&PromptTemplate> {
        self.templates
            .values()
            .filter(|t| &t.category == category)
            .collect()
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
            self.create_backup(current_id, "切换模板前自动备份")?;
        }

        // 激活新模板
        self.active_id = Some(id.to_string());

        // 保存激活状态
        let active_file = self.config_dir.join("active.txt");
        fs::write(active_file, id)?;

        Ok(())
    }

    /// 取消激活
    pub fn deactivate(&mut self) -> Result<()> {
        if let Some(current_id) = &self.active_id {
            self.create_backup(current_id, "取消激活前备份")?;
        }

        self.active_id = None;

        let active_file = self.config_dir.join("active.txt");
        if active_file.exists() {
            fs::remove_file(active_file)?;
        }

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

    /// 创建新模板
    pub fn create_template(&mut self, template: PromptTemplate) -> Result<()> {
        // 验证模板
        if template.id.is_empty() || template.name.is_empty() {
            anyhow::bail!("模板 ID 和名称不能为空");
        }

        if self.templates.contains_key(&template.id) {
            anyhow::bail!("模板 ID 已存在: {}", template.id);
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

    /// 更新模板
    pub fn update_template(&mut self, id: &str, content: String) -> Result<()> {
        let template = self.templates.get_mut(id)
            .context("模板不存在")?;

        // 创建备份
        self.create_backup(id, "更新前自动备份")?;

        // 更新内容
        template.content = content;
        template.updated_at = Utc::now();

        // 保存到文件
        let template_file = self.config_dir
            .join("templates")
            .join(format!("{}.json", id));

        fs::write(template_file, serde_json::to_string_pretty(template)?)?;

        Ok(())
    }

    /// 导入模板
    pub fn import_template(&mut self, template: PromptTemplate) -> Result<()> {
        // 验证模板
        if template.id.is_empty() || template.name.is_empty() {
            anyhow::bail!("模板 ID 和名称不能为空");
        }

        // 如果 ID 已存在，生成新 ID
        let mut final_template = template;
        if self.templates.contains_key(&final_template.id) {
            final_template.id = format!("{}-{}", final_template.id, uuid::Uuid::new_v4());
        }

        // 保存到文件
        let template_file = self.config_dir
            .join("templates")
            .join(format!("{}.json", final_template.id));

        fs::write(template_file, serde_json::to_string_pretty(&final_template)?)?;

        // 添加到内存
        self.templates.insert(final_template.id.clone(), final_template);

        Ok(())
    }

    /// 从文件导入模板
    pub fn import_from_file(&mut self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)?;
        let template: PromptTemplate = serde_json::from_str(&content)?;

        let id = template.id.clone();
        self.import_template(template)?;

        Ok(id)
    }

    /// 导出模板到文件
    pub fn export_template(&self, id: &str, output_path: &Path) -> Result<()> {
        let template = self.templates.get(id)
            .context("模板不存在")?;

        fs::write(output_path, serde_json::to_string_pretty(template)?)?;

        Ok(())
    }

    /// 删除模板
    pub fn delete_template(&mut self, id: &str) -> Result<()> {
        // 不能删除内置模板
        if id == "professional" || id == "creative" || id == "technical" || id == "research" {
            anyhow::bail!("不能删除内置模板");
        }

        // 如果是当前激活的模板，先取消激活
        if self.active_id.as_deref() == Some(id) {
            self.deactivate()?;
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

    /// 删除备份
    pub fn delete_backup(&self, backup_id: &str) -> Result<()> {
        let backup_file = self.config_dir.join("backups").join(format!("{}.json", backup_id));
        if backup_file.exists() {
            fs::remove_file(backup_file)?;
        }
        Ok(())
    }

    /// 清理旧备份（保留最近 N 个）
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize> {
        let mut backups = self.list_backups()?;

        if backups.len() <= keep_count {
            return Ok(0);
        }

        // 已经按时间倒序排序，保留前 keep_count 个
        let to_delete = backups.split_off(keep_count);

        for backup in &to_delete {
            self.delete_backup(&backup.id)?;
        }

        Ok(to_delete.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_system_prompt_manager_new() {
        let temp_dir = tempdir().unwrap();
        let manager = SystemPromptManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 应该有 4 个内置模板
        assert_eq!(manager.list_templates().len(), 4);

        // 检查内置模板
        assert!(manager.get_template("professional").is_some());
        assert!(manager.get_template("creative").is_some());
        assert!(manager.get_template("technical").is_some());
        assert!(manager.get_template("research").is_some());
    }

    #[test]
    fn test_activate_template() {
        let temp_dir = tempdir().unwrap();
        let mut manager = SystemPromptManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 激活模板
        manager.activate("professional").unwrap();
        assert_eq!(manager.active_id(), Some("professional"));

        // 切换模板
        manager.activate("technical").unwrap();
        assert_eq!(manager.active_id(), Some("technical"));

        // 取消激活
        manager.deactivate().unwrap();
        assert_eq!(manager.active_id(), None);
    }

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = tempdir().unwrap();
        let mut manager = SystemPromptManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 激活并创建备份
        manager.activate("professional").unwrap();
        let backup_id = manager.create_backup("professional", "测试备份").unwrap();

        // 列出备份
        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].id, backup_id);

        // 恢复备份
        manager.restore_backup(&backup_id).unwrap();
    }

    #[test]
    fn test_list_by_category() {
        let temp_dir = tempdir().unwrap();
        let manager = SystemPromptManager::new(temp_dir.path().to_path_buf()).unwrap();

        let professional = manager.list_templates_by_category(&PromptCategory::Professional);
        assert_eq!(professional.len(), 1);

        let creative = manager.list_templates_by_category(&PromptCategory::Creative);
        assert_eq!(creative.len(), 1);
    }
}
