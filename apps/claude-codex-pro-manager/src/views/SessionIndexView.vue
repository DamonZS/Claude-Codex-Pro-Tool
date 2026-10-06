<script setup lang="ts">
import { ref, onMounted } from 'vue';
import {
  initSessionIndex,
  scanSessions,
  querySessions,
  getProjectStats,
  type SessionIndex,
  type ProjectStats,
  type SessionFilter,
  type PagedResult,
  formatRelativeTime,
  getSourceLabel,
  getSourceMaskLabel,
} from '../api/sessionIndex';

const isInitialized = ref(false);
const isScanning = ref(false);
const scanResult = ref<any>(null);
const sessions = ref<PagedResult<SessionIndex> | null>(null);
const projectStats = ref<ProjectStats[]>([]);
const filter = ref<SessionFilter>({});
const currentPage = ref(0);
const pageSize = ref(20);
const error = ref<string | null>(null);

// Initialize database
async function initialize() {
  try {
    error.value = null;
    const result = await initSessionIndex();
    console.log('Session index initialized:', result);
    isInitialized.value = true;
    await loadProjectStats();
  } catch (e: any) {
    error.value = `初始化失败: ${e}`;
    console.error('Failed to initialize session index:', e);
  }
}

// Scan sessions
async function scan() {
  try {
    error.value = null;
    isScanning.value = true;
    scanResult.value = await scanSessions();
    console.log('Scan completed:', scanResult.value);
    // Reload data after scan
    await Promise.all([loadSessions(), loadProjectStats()]);
  } catch (e: any) {
    error.value = `扫描失败: ${e}`;
    console.error('Failed to scan sessions:', e);
  } finally {
    isScanning.value = false;
  }
}

// Load sessions
async function loadSessions() {
  try {
    error.value = null;
    sessions.value = await querySessions(filter.value, currentPage.value, pageSize.value);
  } catch (e: any) {
    error.value = `加载会话失败: ${e}`;
    console.error('Failed to load sessions:', e);
  }
}

// Load project stats
async function loadProjectStats() {
  try {
    error.value = null;
    projectStats.value = await getProjectStats();
  } catch (e: any) {
    error.value = `加载项目统计失败: ${e}`;
    console.error('Failed to load project stats:', e);
  }
}

// Filter by project
function filterByProject(project: string) {
  filter.value = { ...filter.value, project };
  currentPage.value = 0;
  loadSessions();
}

// Filter by source
function filterBySource(source: 'claude' | 'codex' | undefined) {
  filter.value = { ...filter.value, source };
  currentPage.value = 0;
  loadSessions();
}

// Search
function search(query: string) {
  filter.value = { ...filter.value, search_query: query || undefined };
  currentPage.value = 0;
  loadSessions();
}

// Clear filters
function clearFilters() {
  filter.value = {};
  currentPage.value = 0;
  loadSessions();
}

// Pagination
function nextPage() {
  if (sessions.value && currentPage.value < sessions.value.total_pages - 1) {
    currentPage.value++;
    loadSessions();
  }
}

function prevPage() {
  if (currentPage.value > 0) {
    currentPage.value--;
    loadSessions();
  }
}

function gotoPage(page: number) {
  currentPage.value = page;
  loadSessions();
}

onMounted(async () => {
  await initialize();
  if (isInitialized.value) {
    await loadSessions();
  }
});
</script>

<template>
  <div class="session-index-view">
    <div class="header">
      <h2>会话索引</h2>
      <div class="actions">
        <button
          @click="scan"
          :disabled="!isInitialized || isScanning"
          class="btn btn-primary"
        >
          {{ isScanning ? '扫描中...' : '扫描会话' }}
        </button>
        <button @click="clearFilters" class="btn btn-secondary">
          清除筛选
        </button>
      </div>
    </div>

    <div v-if="error" class="error-banner">
      {{ error }}
    </div>

    <div v-if="scanResult" class="scan-result">
      <h3>扫描结果</h3>
      <p>已索引文件: {{ scanResult.files_indexed }}</p>
      <p>已索引会话: {{ scanResult.sessions_indexed }}</p>
      <p>已索引消息: {{ scanResult.messages_indexed }}</p>
      <p v-if="scanResult.files_failed > 0" class="error">
        失败文件: {{ scanResult.files_failed }}
      </p>
    </div>

    <div class="content">
      <!-- Project Stats Sidebar -->
      <aside class="sidebar">
        <h3>项目统计</h3>
        <div class="project-list">
          <div
            v-for="proj in projectStats"
            :key="proj.project_name"
            class="project-item"
            :class="{ active: filter.project === proj.project_name }"
            @click="filterByProject(proj.project_name)"
          >
            <div class="project-name">{{ proj.project_name }}</div>
            <div class="project-meta">
              <span>{{ proj.session_count }} 会话</span>
              <span>{{ proj.message_count }} 消息</span>
              <span class="source-badge">{{ getSourceMaskLabel(proj.source_mask) }}</span>
            </div>
            <div class="project-time">
              {{ formatRelativeTime(proj.last_activity) }}
            </div>
          </div>
        </div>

        <div class="source-filter">
          <h4>数据源</h4>
          <button
            @click="filterBySource(undefined)"
            :class="{ active: !filter.source }"
            class="filter-btn"
          >
            全部
          </button>
          <button
            @click="filterBySource('claude')"
            :class="{ active: filter.source === 'claude' }"
            class="filter-btn"
          >
            Claude
          </button>
          <button
            @click="filterBySource('codex')"
            :class="{ active: filter.source === 'codex' }"
            class="filter-btn"
          >
            Codex
          </button>
        </div>
      </aside>

      <!-- Session List -->
      <main class="main-content">
        <div class="search-bar">
          <input
            type="text"
            placeholder="搜索会话标题..."
            @input="(e) => search((e.target as HTMLInputElement).value)"
            class="search-input"
          />
        </div>

        <div v-if="sessions" class="session-list">
          <div
            v-for="session in sessions.items"
            :key="session.session_id"
            class="session-card"
          >
            <div class="session-header">
              <h3 class="session-title">{{ session.title }}</h3>
              <span class="source-badge">{{ getSourceLabel(session.source) }}</span>
            </div>
            <div class="session-meta">
              <span>项目: {{ session.project }}</span>
              <span>消息: {{ session.message_count }}</span>
              <span>{{ formatRelativeTime(session.updated_at) }}</span>
            </div>
            <div class="session-id">ID: {{ session.session_id }}</div>
          </div>

          <div v-if="sessions.items.length === 0" class="empty-state">
            <p>未找到会话</p>
          </div>
        </div>

        <!-- Pagination -->
        <div v-if="sessions && sessions.total_pages > 1" class="pagination">
          <button @click="prevPage" :disabled="currentPage === 0" class="btn-page">
            上一页
          </button>
          <span class="page-info">
            第 {{ currentPage + 1 }} / {{ sessions.total_pages }} 页
            (共 {{ sessions.total }} 条)
          </span>
          <button
            @click="nextPage"
            :disabled="currentPage >= sessions.total_pages - 1"
            class="btn-page"
          >
            下一页
          </button>
        </div>
      </main>
    </div>
  </div>
</template>

<style scoped>
.session-index-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #0B0E14;
  color: #F8FAFC;
  font-family: 'Inter', sans-serif;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.5rem 2rem;
  border-bottom: 1px solid #1E293B;
}

.header h2 {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
}

.actions {
  display: flex;
  gap: 0.75rem;
}

.btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 6px;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: #38BDF8;
  color: #0B0E14;
}

.btn-primary:hover:not(:disabled) {
  background: #22D3EE;
  transform: translateY(-1px);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background: #1E293B;
  color: #F8FAFC;
}

.btn-secondary:hover {
  background: #334155;
}

.error-banner {
  background: #7F1D1D;
  color: #FEE2E2;
  padding: 1rem 2rem;
  border-bottom: 1px solid #991B1B;
}

.scan-result {
  background: #064E3B;
  color: #D1FAE5;
  padding: 1rem 2rem;
  border-bottom: 1px solid #065F46;
}

.scan-result h3 {
  margin: 0 0 0.5rem 0;
  font-size: 1rem;
}

.scan-result p {
  margin: 0.25rem 0;
  font-size: 0.875rem;
}

.content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  width: 280px;
  background: #111827;
  border-right: 1px solid #1E293B;
  overflow-y: auto;
  padding: 1.5rem;
}

.sidebar h3, .sidebar h4 {
  margin: 0 0 1rem 0;
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  color: #94A3B8;
}

.project-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-bottom: 2rem;
}

.project-item {
  padding: 0.75rem;
  background: #1E293B;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
}

.project-item:hover {
  background: #334155;
  transform: translateX(2px);
}

.project-item.active {
  background: #1E3A5F;
  border-color: #38BDF8;
}

.project-name {
  font-weight: 500;
  margin-bottom: 0.25rem;
}

.project-meta {
  display: flex;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: #94A3B8;
  flex-wrap: wrap;
}

.project-time {
  font-size: 0.75rem;
  color: #64748B;
  margin-top: 0.25rem;
}

.source-filter {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.filter-btn {
  padding: 0.5rem;
  background: #1E293B;
  color: #F8FAFC;
  border: 1px solid transparent;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 0.875rem;
}

.filter-btn:hover {
  background: #334155;
}

.filter-btn.active {
  background: #1E3A5F;
  border-color: #38BDF8;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.search-bar {
  padding: 1.5rem 2rem;
  border-bottom: 1px solid #1E293B;
}

.search-input {
  width: 100%;
  padding: 0.75rem 1rem;
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 6px;
  color: #F8FAFC;
  font-size: 0.875rem;
}

.search-input:focus {
  outline: none;
  border-color: #38BDF8;
}

.session-list {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem 2rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.session-card {
  background: #111827;
  border: 1px solid #1E293B;
  border-radius: 8px;
  padding: 1.25rem;
  transition: all 0.2s;
}

.session-card:hover {
  border-color: #38BDF8;
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(56, 189, 248, 0.15);
}

.session-header {
  display: flex;
  justify-content: space-between;
  align-items: start;
  margin-bottom: 0.75rem;
}

.session-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 500;
  flex: 1;
}

.source-badge {
  padding: 0.25rem 0.5rem;
  background: #1E293B;
  border-radius: 4px;
  font-size: 0.75rem;
  color: #38BDF8;
  font-weight: 500;
}

.session-meta {
  display: flex;
  gap: 1rem;
  font-size: 0.875rem;
  color: #94A3B8;
  margin-bottom: 0.5rem;
}

.session-id {
  font-size: 0.75rem;
  color: #64748B;
  font-family: 'Monaco', monospace;
}

.empty-state {
  text-align: center;
  padding: 3rem;
  color: #64748B;
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  padding: 1.5rem 2rem;
  border-top: 1px solid #1E293B;
}

.btn-page {
  padding: 0.5rem 1rem;
  background: #1E293B;
  color: #F8FAFC;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-page:hover:not(:disabled) {
  background: #334155;
}

.btn-page:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.page-info {
  font-size: 0.875rem;
  color: #94A3B8;
}
</style>
