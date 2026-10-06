import { useEffect, useState } from "react";
import { invokeCommand } from "@/tauriBridge";
import type { PagedResult, SessionIndex } from "@/api/sessionIndex";

export function SessionIndexDemo() {
  const [sessions, setSessions] = useState<SessionIndex[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [indexing, setIndexing] = useState(false);

  const loadSessions = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invokeCommand<PagedResult<SessionIndex>>("query_sessions", {
        filter: {},
        page: 0,
        page_size: 20,
      });
      setSessions(result.items);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const startIndexing = async () => {
    setIndexing(true);
    try {
      await invokeCommand("scan_sessions");
      await loadSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIndexing(false);
    }
  };

  useEffect(() => {
    void loadSessions();
  }, []);

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">会话索引（开发演示）</h2>
        <div className="space-x-2">
          <button
            onClick={startIndexing}
            disabled={indexing}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {indexing ? "索引中..." : "重建索引"}
          </button>
          <button
            onClick={loadSessions}
            disabled={loading}
            className="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 disabled:opacity-50"
          >
            刷新
          </button>
        </div>
      </div>

      {error && (
        <div className="p-4 bg-red-100 border border-red-400 text-red-700 rounded">
          {error}
        </div>
      )}

      {loading ? (
        <div className="text-center py-8">加载中...</div>
      ) : (
        <div className="space-y-2">
          <div className="text-sm text-gray-600">共 {sessions.length} 个会话</div>
          {sessions.map((session) => (
            <div
              key={session.session_id}
              className="p-4 border rounded hover:bg-gray-50"
            >
              <div className="font-semibold">{session.title}</div>
              <div className="text-sm text-gray-600">
                项目: {session.project} | 来源: {session.source} |
                消息数: {session.message_count}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                开始: {new Date(session.created_at).toLocaleString()} |
                更新: {new Date(session.updated_at).toLocaleString()}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
