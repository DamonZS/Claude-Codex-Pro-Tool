import { invoke } from '@tauri-apps/api/core';
import type {
  PromptTemplate,
  SystemPromptPayload,
  CommandResult,
} from '../types/system-prompt';

/**
 * 系统提示词服务
 */
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
  static async createTemplate(
    template: PromptTemplate
  ): Promise<SystemPromptPayload> {
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
  static async importTemplate(
    template: PromptTemplate
  ): Promise<SystemPromptPayload> {
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
  static async exportTemplate(
    templateId: string,
    outputPath: string
  ): Promise<void> {
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
