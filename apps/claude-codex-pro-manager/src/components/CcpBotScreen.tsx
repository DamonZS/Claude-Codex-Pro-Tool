import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Bot, Download, ExternalLink, FolderOpen, Play, RefreshCw, Save, Square } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  loadCcpBot,
  prepareCcpBot,
  probeCcpBot,
  readCcpBotLog,
  saveCcpBotConfig,
  startCcpBot,
  stopCcpBot,
} from "@/lib/ccpbot";
import { invokeCommand } from "@/tauriBridge";
import type { CcpBotMode, CcpBotSnapshot } from "@/types";

type Busy = "load" | "prepare" | "probe" | "start" | "stop" | "save" | null;

type Notice = { text: string; ok: boolean };

/**
 * The CCP-Bot workspace.
 *
 * CCP-Bot is a separate bun/docker stack, so this screen is two things at once:
 * a control panel (address, mode, start/stop, health) and the embedded UI itself.
 * The frame is keyed and re-mounted on demand rather than reloaded in place,
 * because a proxied iframe cannot be refreshed from the parent and a stale frame
 * after a restart is the most confusing state this page can be in.
 */
export function CcpBotScreen() {
  const [snapshot, setSnapshot] = useState<CcpBotSnapshot | null>(null);
  const [address, setAddress] = useState("");
  const [mode, setMode] = useState<CcpBotMode>("source");
  const [projectDir, setProjectDir] = useState("");
  const [busy, setBusy] = useState<Busy>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [frameGeneration, setFrameGeneration] = useState(0);
  const [logOpen, setLogOpen] = useState(false);

  const adopt = useCallback((next: CcpBotSnapshot) => {
    setSnapshot(next);
    setAddress(next.config.address);
    setMode(next.config.mode);
    setProjectDir(next.config.projectDir);
  }, []);

  const load = useCallback(async () => {
    setBusy("load");
    try {
      const result = await loadCcpBot();
      adopt(result);
      if (result.status !== "ok") setNotice({ text: result.message, ok: false });
    } catch (error) {
      setNotice({ text: `读取 CCP-Bot 状态失败：${String(error)}`, ok: false });
    } finally {
      setBusy(null);
    }
  }, [adopt]);

  useEffect(() => {
    void load();
  }, [load]);

  const recheck = useCallback(async () => {
    setBusy("probe");
    try {
      const result = await probeCcpBot();
      setSnapshot((previous) =>
        previous
          ? { ...previous, probe: result.probe, runtime: result.runtime, pid: result.pid }
          : previous,
      );
      setNotice({ text: result.message, ok: result.status === "ok" });
      if (logOpen) {
        const log = await readCcpBotLog(120);
        setSnapshot((previous) =>
          previous ? { ...previous, logPath: log.path, logText: log.text } : previous,
        );
      }
    } catch (error) {
      setNotice({ text: `探测失败：${String(error)}`, ok: false });
    } finally {
      setBusy(null);
    }
  }, [logOpen]);

  const save = useCallback(async () => {
    setBusy("save");
    try {
      const result = await saveCcpBotConfig({ address, mode, projectDir });
      setAddress(result.config.address);
      setProjectDir(result.config.projectDir);
      setMode(result.config.mode);
      setNotice({ text: result.message, ok: result.status === "ok" });
      if (result.status === "ok") {
        setSnapshot((previous) => (previous ? { ...previous, config: result.config } : previous));
        setFrameGeneration((value) => value + 1);
      }
    } catch (error) {
      setNotice({ text: `保存失败：${String(error)}`, ok: false });
    } finally {
      setBusy(null);
    }
  }, [address, mode, projectDir]);

  const run = useCallback(
    async (action: "start" | "stop") => {
      setBusy(action);
      try {
        const result = action === "start" ? await startCcpBot() : await stopCcpBot();
        setNotice({ text: result.message, ok: result.status === "ok" });
        // Give the dev server a moment: bun run dev is up in seconds, but the
        // probe that runs immediately after spawn still sees a closed port.
        await new Promise((resolve) => setTimeout(resolve, action === "start" ? 2500 : 800));
        const status = await probeCcpBot();
        setSnapshot((previous) =>
          previous
            ? { ...previous, probe: status.probe, runtime: status.runtime, pid: status.pid }
            : previous,
        );
        setFrameGeneration((value) => value + 1);
      } catch (error) {
        setNotice({ text: `${action === "start" ? "启动" : "停止"}失败：${String(error)}`, ok: false });
      } finally {
        setBusy(null);
      }
    },
    [],
  );

  const download = useCallback(async () => {
    setBusy("prepare");
    try {
      const result = await prepareCcpBot({ address, mode, projectDir });
      setNotice({ text: result.message, ok: result.status === "ok" });
      const next = await loadCcpBot();
      adopt(next);
      setFrameGeneration((value) => value + 1);
    } catch (error) {
      setNotice({ text: `下载并准备失败：${String(error)}`, ok: false });
    } finally {
      setBusy(null);
    }
  }, [adopt, address, mode, projectDir]);

  const openExternal = useCallback(async () => {
    try {
      await invokeCommand("open_external_url", { url: snapshot?.config.address ?? address });
    } catch (error) {
      setNotice({ text: `打开浏览器失败：${String(error)}`, ok: false });
    }
  }, [address, snapshot]);

  const pickDirectory = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 CCP-Bot 源码目录（含 package.json）",
      });
      const picked = Array.isArray(selected) ? selected[0] : selected;
      if (picked) setProjectDir(picked);
    } catch (error) {
      setNotice({ text: `打开目录选择器失败：${String(error)}`, ok: false });
    }
  }, []);

  const reachable = snapshot?.probe.reachable ?? false;
  const runtime = snapshot?.runtime;
  const logText = snapshot?.logText ?? "";

  return (
    <div className="ccpbot-layout">
      <header className="ccpbot-header">
        <div className="ccpbot-title">
          <Bot className="h-5 w-5" />
          <div>
            <h1>CCP-Bot</h1>
            <p>
              {reachable
                ? `已连接 ${snapshot?.config.address ?? ""}`
                : "独立进程的智能体工作台；这里负责启动它并内嵌它的界面。"}
            </p>
          </div>
        </div>
        <div className="ccpbot-chips">
          <span className={`ccpbot-chip ${reachable ? "ok" : "warn"}`}>
            {reachable ? "可达" : "未运行"}
          </span>
          <span className="ccpbot-chip">{mode === "docker" ? "Docker" : "源码"}</span>
          {snapshot?.pid ? <span className="ccpbot-chip">pid {snapshot.pid}</span> : null}
          <span className="ccpbot-chip">
            {runtime?.bunVersion ? `bun ${runtime.bunVersion}` : "bun 未找到"}
          </span>
          <span className={`ccpbot-chip ${runtime?.dockerDaemon ? "ok" : ""}`}>
            {runtime?.dockerDaemon ? "docker 就绪" : "docker daemon 未启动"}
          </span>
        </div>
      </header>

      <section className="ccpbot-controls">
        <label className="ccpbot-field">
          <span>地址</span>
          <Input
            onChange={(event) => setAddress(event.target.value)}
            placeholder="http://127.0.0.1:3010"
            value={address}
          />
        </label>
        <label className="ccpbot-field">
          <span>启动方式</span>
          <select
            className="ccpbot-select"
            onChange={(event) => setMode(event.target.value as CcpBotMode)}
            value={mode}
          >
            <option value="source">源码（bun run dev）</option>
            <option value="docker">Docker（compose up -d）</option>
          </select>
        </label>
        <label className="ccpbot-field ccpbot-field-wide">
          <span>项目目录</span>
          <Input
            onChange={(event) => setProjectDir(event.target.value)}
            placeholder="例如 D:\\CCP-Bot（空文件夹，或已有的 CCP-Bot 目录）"
            value={projectDir}
          />
        </label>
        <div className="ccpbot-buttons">
          <Button disabled={busy !== null} onClick={() => void run("start")} variant="default">
            <Play className="h-4 w-4" />
            启动
          </Button>
          <Button disabled={busy !== null} onClick={() => void run("stop")} variant="outline">
            <Square className="h-4 w-4" />
            停止
          </Button>
          <Button disabled={busy !== null} onClick={() => void recheck()} variant="ghost">
            <RefreshCw className="h-4 w-4" />
            重新检测
          </Button>
          <Button disabled={busy !== null} onClick={() => void pickDirectory()} variant="ghost">
            <FolderOpen className="h-4 w-4" />
            选择目录
          </Button>
          <Button disabled={busy !== null} onClick={() => void download()} variant="secondary">
            <Download className="h-4 w-4" />
            {busy === "prepare" ? "下载中…" : "下载并准备"}
          </Button>          <Button disabled={busy !== null} onClick={() => void save()} variant="ghost">
            <Save className="h-4 w-4" />
            保存
          </Button>
          <Button disabled={busy !== null} onClick={() => void openExternal()} variant="ghost">
            <ExternalLink className="h-4 w-4" />
            浏览器打开
          </Button>
        </div>
      </section>

      {notice ? (
        <p className={`ccpbot-notice ${notice.ok ? "ok" : "warn"}`}>{notice.text}</p>
      ) : null}

      {projectDir.trim() === "" ? (
        <p className="ccpbot-notice">
          还没选 CCP-Bot 目录。想直接用已在跑的实例就把地址填对、不用填目录；本机压根没有 CCP-Bot 的话，点「选择目录」挑一个空文件夹，再「下载并准备」——它会从 GitHub 拉中文化版本、装好依赖并生成 .env。
        </p>
      ) : runtime && !runtime.projectDirPresent ? (
        <p className="ccpbot-notice warn">
          这个目录下没有 package.json 与 docker-compose.yml，不像 CCP-Bot 源码目录。
        </p>
      ) : null}
      {runtime && mode === "source" && !runtime.envReady && runtime.projectDirPresent ? (
        <p className="ccpbot-notice warn">
          源码模式需要 .env：先把 .env.example 复制成 .env，再点启动。
        </p>
      ) : null}

      <div className="ccpbot-frame-wrap">
        {snapshot && reachable ? (
          <iframe
            allow="clipboard-read; clipboard-write; microphone; camera"
            className="ccpbot-frame"
            key={`${snapshot.config.address}-${frameGeneration}`}
            referrerPolicy="no-referrer"
            src={snapshot.config.address}
            title="CCP-Bot"
          />
        ) : (
          <div className="ccpbot-placeholder">
            <Bot className="h-8 w-8" />
            <h2>CCP-Bot 还没有响应</h2>
            <p>{snapshot?.probe.detail ?? "正在读取状态…"}</p>
            <div className="ccpbot-buttons">
              <Button disabled={busy !== null} onClick={() => void run("start")}>
                <Play className="h-4 w-4" />
                启动 CCP-Bot
              </Button>
              <Button disabled={busy !== null} onClick={() => void recheck()} variant="outline">
                <RefreshCw className="h-4 w-4" />
                重新检测
              </Button>
            </div>
            <p className="ccpbot-hint">
              首次启动要等 Vite 与 server 就绪（约十几秒），就绪后这里会自动换成内嵌界面。
            </p>
          </div>
        )}
      </div>

      <details
        className="ccpbot-log"
        onToggle={(event) => {
          const open = (event.target as HTMLDetailsElement).open;
          setLogOpen(open);
          if (open) void recheck();
        }}
      >
        <summary>
          启动日志 · {snapshot?.logPath ?? "尚未启动"}
        </summary>
        <pre className="ops-code">{logText || "暂无日志。"}</pre>
      </details>
    </div>
  );
}