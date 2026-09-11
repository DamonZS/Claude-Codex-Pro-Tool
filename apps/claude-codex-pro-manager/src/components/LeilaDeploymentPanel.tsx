import { useEffect, useRef, useState } from "react";
import {
  CheckCircle2,
  FolderOpen,
  LoaderCircle,
  PackageCheck,
  RefreshCw,
  RotateCcw,
  ScrollText,
  ShieldCheck,
  TriangleAlert,
  Wifi,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import { statusOk } from "@/lib/helpers";
import type { LeilaDeploymentResult, LeilaDeploymentStatus } from "@/types";

type Operation = "inspect" | "choose" | "deploy" | "rollback";

type Props = {
  actions: AppActions;
  status: LeilaDeploymentStatus | null;
};

const MODULE_STATUS_LABELS: Record<string, string> = {
  not_checked: "未检测",
  pending: "待安装",
  installing: "安装中",
  complete: "已完成",
  failed: "失败",
};

function compactHash(status: LeilaDeploymentStatus | null) {
  const hash = status?.resourceSha256 || status?.promptSha256 || status?.acSha256;
  if (!hash) return "尚无校验记录";
  return hash.length > 24 ? `${hash.slice(0, 12)}...${hash.slice(-12)}` : hash;
}

function formatDeploymentTime(value: string | number | null | undefined) {
  if (!value) return "尚未部署";
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString("zh-CN", { hour12: false });
}

function moduleStatusLabel(status: LeilaDeploymentStatus | null, operation: Operation | null) {
  if (operation === "deploy") return "安装中";
  if (!status) return "未检测";
  if (status.pythonModuleStatus) return MODULE_STATUS_LABELS[status.pythonModuleStatus] ?? status.pythonModuleStatus;
  if (!status.pythonVersion) return "待安装";
  return status.pythonModulesInstalled ? "已完成" : "待安装";
}

function deploymentState(status: LeilaDeploymentStatus | null) {
  if (!status) return { label: "检测中", tone: "muted" };
  if (!statusOk(status.status)) return { label: "检测失败", tone: "warn" };
  if (!status.supported) return { label: "不支持", tone: "warn" };
  if (status.externallyModified) return { label: "外部修改", tone: "warn" };
  if (status.operationManifest && (!status.promptVerified || !status.identityVerified || !status.acVerified || !status.globalProfileActive)) {
    return { label: "资源不匹配", tone: "warn" };
  }
  if (status.deployed) return { label: status.rollbackAvailable ? "已部署，可回滚" : "已部署", tone: "ok" };
  return { label: status.rollbackAvailable ? "可回滚" : "未部署", tone: "muted" };
}

export function LeilaDeploymentPanel({ actions, status }: Props) {
  const [operation, setOperation] = useState<Operation | null>(null);
  const [confirmMode, setConfirmMode] = useState<"deploy" | "rollback" | null>(null);
  const [logsOpen, setLogsOpen] = useState(false);
  const logViewportRef = useRef<HTMLPreElement>(null);
  const busy = operation !== null;
  const state = deploymentState(status);
  const logs = status?.logs ?? [];
  const supported = status?.supported === true;
  const target = status?.targetCodexHome;
  const targetMissingConfig = logs.some((line) => line.includes("目标目录缺少 config.toml"));

  useEffect(() => {
    if (!logsOpen) return;
    const viewport = logViewportRef.current;
    if (viewport) viewport.scrollTop = viewport.scrollHeight;
  }, [logs, logsOpen]);

  const runStatusAction = async (kind: "inspect" | "choose", action: () => Promise<LeilaDeploymentStatus | null>) => {
    setOperation(kind);
    try {
      await action();
    } finally {
      setOperation(null);
    }
  };

  const runDeploymentAction = async (kind: "deploy" | "rollback", action: () => Promise<LeilaDeploymentResult | null>) => {
    setLogsOpen(true);
    setOperation(kind);
    try {
      await action();
    } finally {
      setOperation(null);
      setConfirmMode(null);
    }
  };

  return (
    <section className="leila-deployment-panel" aria-labelledby="leila-deployment-title" aria-busy={busy}>
      <header className="leila-deployment-header">
        <div className="leila-deployment-title">
          <span className="leila-deployment-icon"><PackageCheck aria-hidden="true" /></span>
          <div>
            <p>破甲</p>
            <h2 id="leila-deployment-title">破甲部署</h2>
          </div>
        </div>
        <span className={`leila-deployment-state is-${state.tone}`}>
          {state.tone === "ok" ? <CheckCircle2 aria-hidden="true" /> : state.tone === "warn" ? <TriangleAlert aria-hidden="true" /> : null}
          {state.label}
        </span>
      </header>

      <div className="leila-deployment-facts">
        <div><span>资源版本</span><strong>{status?.packageVersion || "1.0.7"}</strong></div>
        <div><span>运行环境</span><strong>{status ? `${status.platform}/${status.architecture}` : "检测中"}</strong></div>
        <div><span>Python</span><strong>{status?.pythonVersion ? `${status.pythonVersion}${status.pythonBits ? ` / ${status.pythonBits} 位` : ""}` : "未检测到"}</strong></div>
        <div><span>Python 模块</span><strong>{moduleStatusLabel(status, operation)}</strong></div>
        <div className="is-wide"><span>Codex 目标目录</span><strong title={target || undefined}>{target || "未找到包含 config.toml 的 Codex 目录"}</strong></div>
        <div><span>全局指令</span><strong>{status?.globalProfileActive ? "破甲已激活" : "未激活"}</strong></div>
        <div><span>最近部署</span><strong>{status?.lastResult ? `${formatDeploymentTime(status.lastDeploymentAt)} / ${status.lastResult}` : formatDeploymentTime(status?.lastDeploymentAt)}</strong></div>
        <div className="is-wide"><span>最近资源 SHA-256</span><strong className="is-mono" title={status?.resourceSha256 || status?.promptSha256 || undefined}>{compactHash(status)}</strong></div>
      </div>

      {!status ? (
        <div className="leila-deployment-message"><LoaderCircle className="spin" aria-hidden="true" />正在检测破甲环境和部署状态</div>
      ) : !supported ? (
        <div className="leila-deployment-message is-warning"><TriangleAlert aria-hidden="true" />不支持当前破甲部署包，仅支持 Windows/macOS 64 位。</div>
      ) : targetMissingConfig ? (
        <div className="leila-deployment-message is-warning"><TriangleAlert aria-hidden="true" />目标目录缺少 config.toml，请先选择有效的 Codex 目录。</div>
      ) : status.lastError ? (
        <div className="leila-deployment-message is-error">
          <TriangleAlert aria-hidden="true" />
          <span>
            <strong>最近操作失败</strong>
            {status.lastError}
            {logs.length > 0 ? <code>{logs.slice(-3).join("\n")}</code> : null}
          </span>
        </div>
      ) : (
        <div className="leila-deployment-message"><Wifi aria-hidden="true" />提示词和 Skill 从 CCP 本地资源部署；Python 模块安装需要网络连接。</div>
      )}

      <footer className="leila-deployment-actions">
        <Button variant="outline" disabled={busy} onClick={() => void runStatusAction("inspect", () => actions.refreshLeilaStatus(false))}>
          {operation === "inspect" ? <LoaderCircle className="spin" aria-hidden="true" /> : <RefreshCw aria-hidden="true" />}检测环境
        </Button>
        <Button variant="outline" disabled={busy} onClick={() => void runStatusAction("choose", actions.chooseLeilaTarget)}>
          {operation === "choose" ? <LoaderCircle className="spin" aria-hidden="true" /> : <FolderOpen aria-hidden="true" />}选择 Codex 目录
        </Button>
        <Button disabled={busy || !supported || !target || targetMissingConfig} onClick={() => setConfirmMode("deploy")}>
          <ShieldCheck aria-hidden="true" />部署破甲
        </Button>
        <Button variant="outline" disabled={busy || !status?.rollbackAvailable} onClick={() => setConfirmMode("rollback")}>
          <RotateCcw aria-hidden="true" />回滚最近一次
        </Button>
        <Button variant="ghost" disabled={logs.length === 0} aria-expanded={logsOpen} onClick={() => setLogsOpen((open) => !open)}>
          <ScrollText aria-hidden="true" />{logsOpen ? "收起部署日志" : "查看部署日志"}
        </Button>
      </footer>

      {logsOpen ? (
        <section className="leila-deployment-log-section" aria-labelledby="leila-logs-title">
          <header>
            <div><small>操作记录</small><strong id="leila-logs-title">部署日志</strong></div>
            <span>{logs.length} 条</span>
          </header>
          <pre ref={logViewportRef} className="leila-deployment-logs" tabIndex={0}>{logs.join("\n")}</pre>
        </section>
      ) : null}

      {confirmMode === "deploy" ? (
        <div className="system-prompt-modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) setConfirmMode(null); }}>
          <section className="system-prompt-modal is-compact" role="dialog" aria-modal="true" aria-labelledby="leila-deploy-confirm-title">
            <header>
              <div><small>DEPLOY ARMOR BREAK</small><h2 id="leila-deploy-confirm-title">确认部署破甲</h2></div>
              <button type="button" title="关闭" disabled={busy} onClick={() => setConfirmMode(null)}><X /></button>
            </header>
            <div className="leila-deployment-confirm">
              <p>将先联网安装匹配当前 Python 版本的模块，再备份并修改以下资源：</p>
              <code>{target}\config.toml</code>
              <code>{target}\gpt5.5-unrestricted.md</code>
              <code>{target}\skills\leila-identity</code>
              <code>{target}\skills\ac</code>
              <p>不会修改 <code>hooks\instruction-inject.sh</code>、provider 或 model。</p>
            </div>
            <footer>
              <Button variant="outline" disabled={busy} onClick={() => setConfirmMode(null)}>取消</Button>
              <Button disabled={busy} onClick={() => void runDeploymentAction("deploy", actions.deployLeila)}>
                {operation === "deploy" ? <LoaderCircle className="spin" aria-hidden="true" /> : <ShieldCheck aria-hidden="true" />}
                {operation === "deploy" ? "正在部署破甲" : "确认部署破甲"}
              </Button>
            </footer>
          </section>
        </div>
      ) : null}

      {confirmMode === "rollback" ? (
        <div className="system-prompt-modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) setConfirmMode(null); }}>
          <section className="system-prompt-modal is-compact" role="dialog" aria-modal="true" aria-labelledby="leila-rollback-confirm-title">
            <header>
              <div><small>ROLLBACK ARMOR BREAK</small><h2 id="leila-rollback-confirm-title">回滚最近一次破甲部署</h2></div>
              <button type="button" title="关闭" disabled={busy} onClick={() => setConfirmMode(null)}><X /></button>
            </header>
            <div className="leila-deployment-confirm">
              <p>将按最近一次 CCP 部署清单恢复配置、提示词和两个 Skill。历史备份与回滚清单会保留。</p>
              <code>{target}</code>
            </div>
            <footer>
              <Button variant="outline" disabled={busy} onClick={() => setConfirmMode(null)}>取消</Button>
              <Button disabled={busy} onClick={() => void runDeploymentAction("rollback", actions.rollbackLeila)}>
                {operation === "rollback" ? <LoaderCircle className="spin" aria-hidden="true" /> : <RotateCcw aria-hidden="true" />}
                {operation === "rollback" ? "正在回滚" : "确认回滚"}
              </Button>
            </footer>
          </section>
        </div>
      ) : null}

    </section>
  );
}
