export type MemoryKind = "persona" | "memory"; // persona=画像，memory=任务记忆
export type MemoryFilter = "all" | MemoryKind;

export type MemoryLibraryEntry = {
  id: string;
  kind: MemoryKind;
  title: string;
  body: string;
  source: string;      // 例如 "distill"
  createdAt: string;   // ISO 或毫秒字符串，组件内用 new Date(Number(v)) 兼容
  updatedAt: string;
};

export type MemoryLibraryCounts = { total: number; persona: number; memory: number };

export type MemorySaveInput = { id: string | null; kind: MemoryKind; title: string; body: string };
