/**
 * 系统提示词相关类型定义
 */

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

export const CATEGORY_LABELS: Record<PromptCategory, string> = {
  professional: '专业模式',
  creative: '创意模式',
  technical: '技术模式',
  research: '研究模式',
  custom: '自定义',
};

export const CATEGORY_COLORS: Record<PromptCategory, string> = {
  professional: 'blue',
  creative: 'purple',
  technical: 'green',
  research: 'orange',
  custom: 'gray',
};

export const CATEGORY_DESCRIPTIONS: Record<PromptCategory, string> = {
  professional: '准确、可靠的专业回答',
  creative: '激发创造力和创新思维',
  technical: '专注于代码和技术分析',
  research: '深入分析和系统性研究',
  custom: '用户自定义模板',
};
