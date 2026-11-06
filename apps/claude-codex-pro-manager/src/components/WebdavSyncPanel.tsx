import { useCallback, useEffect, useState } from "react";
import { CloudDownload, CloudUpload, Save } from "lucide-react";

import { Button } from "@/components/ui/button";
import { InfoRow, Panel } from "@/components/ui/ops";
import type { AppActions } from "@/lib/actions";
import type { WebdavSyncPayload } from "@/types";

/**
 * 「WebDAV 配置备份」面板：保存 WebDAV 账号，上传 / 拉取本机配置备份。
 * 数据来自 `load/save/upload/download_webdav_sync` 系列命令，`null` 表示本次操作失败。
 */
export function WebdavSyncPanel({ actions }: { actions: AppActions }) {
  const [baseUrl, setBaseUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [payload, setPayload] = useState<WebdavSyncPayload | null>(null);
  const [pending, setPending] = useState<string | null>(null);

  const applyPayload = useCallback((next: WebdavSyncPayload) => {
    setBaseUrl(next.baseUrl ?? "");
    setUsername(next.username ?? "");
    setPassword(next.password ?? "");
    setPayload(next);
  }, []);

  useEffect(() => {
    let cancelled = false;
    setPending("load");
    void (async () => {
      try {
        const result = await actions.loadWebdavSync();
        if (!cancelled && result) applyPayload(result);
      } finally {
        if (!cancelled) setPending(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [actions, applyPayload]);

  const runAction = async (key: string, task: () => Promise<WebdavSyncPayload | null>) => {
    setPending(key);
    try {
      const result = await task();
      if (result) applyPayload(result);
      return result;
    } finally {
      setPending(null);
    }
  };

  const save = () => runAction("save", () => actions.saveWebdavSync(baseUrl.trim(), username.trim(), password));

  const upload = () => runAction("upload", () => actions.uploadWebdavBackup());

  const download = () => {
    const fileName = payload?.lastFile ?? "";
    if (!fileName) {
      actions.showNotice({
        title: "WebDAV 配置备份",
        message: "本机还没有上传过备份，请先点『立即上传备份』。",
        status: "failed",
      });
      return Promise.resolve(null);
    }
    return runAction("download", () => actions.downloadWebdavBackup(fileName));
  };

  const busy = pending !== null;

  return (
    <Panel title="WebDAV 配置备份" detail="把本机配置备份上传到自己的 WebDAV 目录，换机时再从远端拉回。密码只保存在本机配置里。">
      <div className="stack">
        <label className="ops-form-field">
          <span>WebDAV 地址</span>
          <input
            autoComplete="off"
            disabled={busy}
            onChange={(event) => setBaseUrl(event.target.value)}
            placeholder="https://dav.example.com/dav/ccp"
            value={baseUrl}
          />
        </label>
        <label className="ops-form-field">
          <span>用户名</span>
          <input
            autoComplete="off"
            disabled={busy}
            onChange={(event) => setUsername(event.target.value)}
            placeholder="WebDAV 用户名"
            value={username}
          />
        </label>
        <label className="ops-form-field">
          <span>密码</span>
          <input
            autoComplete="off"
            disabled={busy}
            onChange={(event) => setPassword(event.target.value)}
            placeholder="WebDAV 密码"
            type="password"
            value={password}
          />
        </label>
        <div className="action-row">
          <Button disabled={busy} onClick={() => void save()} size="sm">
            <Save className="h-4 w-4" />
            {pending === "save" ? "保存中…" : "保存配置"}
          </Button>
          <Button disabled={busy} onClick={() => void upload()} size="sm" variant="outline">
            <CloudUpload className="h-4 w-4" />
            {pending === "upload" ? "上传中…" : "立即上传备份"}
          </Button>
          <Button disabled={busy} onClick={() => void download()} size="sm" variant="outline">
            <CloudDownload className="h-4 w-4" />
            {pending === "download" ? "拉取中…" : "从远端拉取"}
          </Button>
        </div>
        {payload?.lastFile ? <InfoRow label="上次上传文件" value={payload.lastFile} /> : null}
        {payload?.remoteUrl ? <InfoRow label="本次上传地址" value={payload.remoteUrl} /> : null}
      </div>
    </Panel>
  );
}