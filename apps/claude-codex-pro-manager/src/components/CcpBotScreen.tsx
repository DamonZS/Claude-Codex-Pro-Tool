import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Bot,
  Check,
  Download,
  ExternalLink,
  FolderOpen,
  PackagePlus,
  Play,
  RefreshCw,
  Save,
  ScrollText,
  Settings2,
  Square,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  installCcpBotLatest,
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

type Busy = "install" | "load" | "prepare" | "probe" | "start" | "stop" | "save" | null;

type Notice = { text: string; ok: boolean };

/**
 * The CCP-Bot workspace, arranged as one status rail over one stage.
 *
 * The page has a single job: show the embedded product, or explain in one glance
 * why it cannot be shown yet. Everything that is configuration rather than
 * operation — address, launch mode, project directory — lives in a sheet that
 * stays closed, because a wall of controls over an empty frame is exactly the
 * state this page was rebuilt to stop showing. The frame is keyed and re-mounted
 * on demand rather than reloaded in place, since a proxied iframe cannot be
 * refreshed from the parent and a stale frame after a restart is the most
 * confusing state this page can be in.
 */
export function CcpBotScreen() {
  const [snapshot, setSnapshot] = useState<CcpBotSnapshot | null>(null);
  const [address, setAddress] = useState("");
  const [mode, setMode] = useState<CcpBotMode>("source");
  const [projectDir, setProjectDir] = useState("");
  const [busy, setBusy] = useState<Busy>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [frameGeneration, setFrameGeneration] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
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

  const install = useCallback(async () => {
    setBusy("install");
    try {
      // No address, no directory: this path exists for the machine that has no
      // CCP-Bot yet, and the release decides which bundle fits this computer. The
      // installer runs while the call is in flight, so there is nothing to poll
      // afterwards — only the message it returns.
      const result = await installCcpBotLatest();
      setNotice({ text: result.message, ok: result.status === "ok" });
    } catch (error) {
      setNotice({ text: `安装最新版失败：${String(error)}`, ok: false });
    } finally {
      setBusy(null);
    }
  }, []);

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

  const readLog = useCallback(async () => {
    try {
      const log = await readCcpBotLog(120);
      setSnapshot((previous) =>
        previous ? { ...previous, logPath: log.path, logText: log.text } : previous,
      );
    } catch (error) {
      setNotice({ text: `读取日志失败：${String(error)}`, ok: false });
    }
  }, []);

  const toggleLog = useCallback(() => {
    setLogOpen((previous) => {
      const next = !previous;
      // The tail is read on the click that opens the drawer, by this function and
      // not by `recheck`: `recheck` closes over `logOpen`, so the click that flips
      // it to true would still be read through a stale `false` and the drawer would
      // open empty. While the drawer stays open, `recheck` does refresh it.
      if (next) void readLog();
      return next;
    });
  }, [readLog]);

  const reachable = snapshot?.probe.reachable ?? false;
  const runtime = snapshot?.runtime;
  const logText = snapshot?.logText ?? "";
  const starting = busy === "start";
  const loading = busy === "load" && snapshot === null;

  // The checklist is the empty state's whole point: it names the one step that
  // is missing instead of printing every requirement and letting the reader
  // guess which of them is the reason nothing is running.
  const hasDirectory = projectDir.trim() !== "";
  const engineReady = mode === "source" ? Boolean(runtime?.bunVersion) : Boolean(runtime?.dockerDaemon);
  const envReady = mode === "source" ? Boolean(runtime?.envReady) : true;
  const steps = [
    {
      label: "取得 CCP-Bot",
      detail: hasDirectory
        ? "目录已就绪"
        : "装最新版，或挑一个空文件夹再「下载并准备」",
      done: hasDirectory,
    },
    {
      label: mode === "source" ? "本机 bun" : "Docker 引擎",
      detail:
        mode === "source"
          ? runtime?.bunVersion
            ? `bun ${runtime.bunVersion}`
            : "没有 bun，源码模式跑不起来"
          : runtime?.dockerDaemon
            ? "daemon 运行中"
            : "daemon 未启动，先打开 Docker Desktop",
      done: engineReady,
    },
    {
      label: "配置",
      detail:
        mode === "source"
          ? runtime?.envReady
            ? ".env 已就绪"
            : "缺 .env：把 .env.example 复制成 .env"
          : "compose 自带环境变量",
      done: envReady,
    },
  ];
  const missing = steps.filter((step) => !step.done);
  const blocked = hasDirectory && !engineReady;

  return (
    <div className="ccpbot-layout">
      <header className="ccpbot-rail">
        <div className="ccpbot-ident">
          <span className={reachable ? "ccpbot-orb live" : starting ? "ccpbot-orb waking" : "ccpbot-orb"}>
            <Bot className="h-4 w-4" />
          </span>
          <div className="ccpbot-ident-text">
            <div className="ccpbot-ident-line">
              <h1>CCP-Bot</h1>
              <code className="ccpbot-addr">{snapshot?.config.address ?? address}</code>
            </div>
            <p className="ccpbot-sub">
              {reachable
                ? "已连上内嵌界面"
                : starting
                  ? "正在启动，Vite 与 server 就绪后这里会自动换成内嵌界面"
                  : "独立进程的智能体工作台"}
            </p>
          </div>
        </div>

        <div className="ccpbot-stats">
          <span className="ccpbot-stat">
            <b>{reachable ? "在线" : starting ? "启动中" : "离线"}</b>
            <i>状态</i>
          </span>
          <span className="ccpbot-stat">
            <b>{mode === "docker" ? "Docker" : "源码"}</b>
            <i>方式</i>
          </span>
          {snapshot?.pid ? (
            <span className="ccpbot-stat">
              <b>{snapshot.pid}</b>
              <i>pid</i>
            </span>
          ) : null}
          <span className="ccpbot-stat">
            <b>{runtime?.bunVersion ?? "—"}</b>
            <i>bun</i>
          </span>
          {mode === "docker" ? (
            <span className={`ccpbot-stat ${runtime?.dockerDaemon ? "" : "off"}`}>
              <b>{runtime?.dockerDaemon ? "就绪" : "未启动"}</b>
              <i>daemon</i>
            </span>
          ) : null}
        </div>

        <div className="ccpbot-actions">
          <div className="ccpbot-segmented">
            <Button disabled={busy !== null} onClick={() => void run("start")} size="sm" variant="default">
              <Play className="h-4 w-4" />
              {starting ? "启动中" : "启动"}
            </Button>
            <Button disabled={busy !== null} onClick={() => void run("stop")} size="sm" variant="outline">
              <Square className="h-4 w-4" />
              停止
            </Button>
          </div>
          <div className="ccpbot-iconline">
            <Button
              aria-label="重新检测"
              disabled={busy !== null}
              onClick={() => void recheck()}
              size="icon"
              title="重新检测"
              variant="ghost"
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
            <Button
              aria-label="保存配置"
              disabled={busy !== null}
              onClick={() => void save()}
              size="icon"
              title="保存配置"
              variant="ghost"
            >
              <Save className="h-4 w-4" />
            </Button>
            <Button
              aria-label="在浏览器打开"
              disabled={busy !== null}
              onClick={() => void openExternal()}
              size="icon"
              title="在浏览器打开"
              variant="ghost"
            >
              <ExternalLink className="h-4 w-4" />
            </Button>
            <Button
              aria-label="连接设置"
              data-open={settingsOpen}
              onClick={() => setSettingsOpen((previous) => !previous)}
              size="icon"
              title="连接设置"
              variant="ghost"
            >
              <Settings2 className="h-4 w-4" />
            </Button>
            <Button
              aria-label="启动日志"
              data-open={logOpen}
              onClick={toggleLog}
              size="icon"
              title="启动日志"
              variant="ghost"
            >
              <ScrollText className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </header>

      <section className="ccpbot-sheet" data-open={settingsOpen}>
        <div className="ccpbot-sheet-inner">
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
          <div className="ccpbot-sheet-actions">
            <Button disabled={busy !== null} onClick={() => void pickDirectory()} size="sm" variant="outline">
              <FolderOpen className="h-4 w-4" />
              选择目录
            </Button>
            <Button disabled={busy !== null} onClick={() => void save()} size="sm" variant="ghost">
              <Save className="h-4 w-4" />
              {busy === "save" ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </section>

      <section className="ccpbot-stage" data-live={reachable}>
        {/* The stage caption only exists while the frame does. When nothing is
            running the empty state is already the explanation, and a second
            sentence of status above it just repeats it in a smaller font. */}
        {reachable ? (
          <div className="ccpbot-stage-head">
            <span className="ccpbot-stage-title">内嵌界面</span>
            <span className="ccpbot-stage-detail">
              来自 {snapshot?.config.address ?? address}；重启后会重新挂载一次。
            </span>
          </div>
        ) : null}

        <div className="ccpbot-frame-wrap">
          {loading ? (
            <div aria-hidden className="ccpbot-skeleton">
              <span />
              <span />
              <span />
            </div>
          ) : reachable && snapshot ? (
            <iframe
              allow="clipboard-read; clipboard-write; microphone; camera"
              className="ccpbot-frame"
              key={`${snapshot.config.address}-${frameGeneration}`}
              referrerPolicy="no-referrer"
              src={snapshot.config.address}
              title="CCP-Bot"
            />
          ) : (
            <div className="ccpbot-empty">
              <div className="ccpbot-empty-mark">
                <span className={starting ? "ccpbot-orb waking" : "ccpbot-orb"}>
                  <Bot className="h-6 w-6" />
                </span>
                <h2>{starting ? "正在唤醒 CCP-Bot" : "CCP-Bot 还没跑起来"}</h2>
                {!starting && snapshot ? (
                  <p className="ccpbot-empty-reason">{snapshot.probe.detail}</p>
                ) : null}
                <p>
                  {starting
                    ? "Vite 与 server 大概十几秒，就绪后这里会自动换成内嵌界面。"
                    : missing.length === 0
                      ? "三样都齐了，点一下按钮就行。"
                      : `还差 ${missing.length} 步：`}
                </p>
              </div>

              <ul className="ccpbot-steps">
                {steps.map((step, index) => (
                  <li
                    className={step.done ? "ccpbot-step done" : "ccpbot-step"}
                    key={step.label}
                    style={{ ["--i" as string]: index }}
                  >
                    <span className="ccpbot-step-mark">{step.done ? <Check className="h-3 w-3" /> : null}</span>
                    <span className="ccpbot-step-label">{step.label}</span>
                    <span className="ccpbot-step-detail">{step.detail}</span>
                  </li>
                ))}
              </ul>

              <div className="ccpbot-empty-actions">
                <Button
                  disabled={busy !== null || blocked}
                  onClick={() => void run("start")}
                  title={blocked ? "先补上缺的那一步" : undefined}
                >
                  <Play className="h-4 w-4" />
                  {starting ? "启动中…" : "启动 CCP-Bot"}
                </Button>
                <Button disabled={busy !== null} onClick={() => void recheck()} variant="outline">
                  <RefreshCw className="h-4 w-4" />
                  重新检测
                </Button>
              </div>

              <div className="ccpbot-empty-alt">
                <span>本机还没有 CCP-Bot：</span>
                <Button disabled={busy !== null} onClick={() => void install()} size="sm" variant="ghost">
                  <PackagePlus className="h-4 w-4" />
                  {busy === "install" ? "安装中…" : "安装最新版"}
                </Button>
                <Button disabled={busy !== null} onClick={() => void download()} size="sm" variant="ghost">
                  <Download className="h-4 w-4" />
                  {busy === "prepare" ? "下载中…" : "下载并准备"}
                </Button>
                <Button disabled={busy !== null} onClick={() => void pickDirectory()} size="sm" variant="ghost">
                  <FolderOpen className="h-4 w-4" />
                  选择目录
                </Button>
              </div>
            </div>
          )}

          {starting ? <div aria-hidden className="ccpbot-progress" /> : null}
        </div>
      </section>

      <section className="ccpbot-log-drawer" data-open={logOpen}>
        <div className="ccpbot-log-head">
          <span>启动日志</span>
          <code>{snapshot?.logPath ?? "尚未启动"}</code>
          <Button
            aria-label="收起日志"
            onClick={() => setLogOpen(false)}
            size="icon"
            title="收起日志"
            variant="ghost"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
        <pre className="ops-code">{logText || "暂无日志。"}</pre>
      </section>

      {notice ? (
        <div className={notice.ok ? "ccpbot-toast ok" : "ccpbot-toast warn"} role="status">
          <span className="ccpbot-toast-mark">{notice.ok ? <Check className="h-3 w-3" /> : "!"}</span>
          <p>{notice.text}</p>
          <Button aria-label="关闭提示" onClick={() => setNotice(null)} size="icon" variant="ghost">
            <X className="h-4 w-4" />
          </Button>
        </div>
      ) : null}
    </div>
  );
}