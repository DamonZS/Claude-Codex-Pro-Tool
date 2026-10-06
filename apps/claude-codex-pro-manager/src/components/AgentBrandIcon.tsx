import claudeCodeLogo from "@/assets/agent-brands/claude-code.svg";
import codexLogo from "@/assets/agent-brands/codex.svg";
import copilotLogo from "@/assets/agent-brands/copilot.svg";
import cursorLogo from "@/assets/agent-brands/cursor.svg";
import deepseekLogo from "@/assets/agent-brands/deepseek.svg";
import kiloLogo from "@/assets/agent-brands/kilo.svg";
import kimiLogo from "@/assets/agent-brands/kimi.svg";
import openclawLogo from "@/assets/agent-brands/openclaw.svg";
import qoderLogo from "@/assets/agent-brands/qoder-cn.svg";
import rooLogo from "@/assets/agent-brands/roo.ico";
import workbuddyLogo from "@/assets/agent-brands/workbuddy.svg";

const logos: Record<string, string> = {
  claude: claudeCodeLogo,
  "claude-code": claudeCodeLogo,
  "claude-desktop": claudeCodeLogo,
  codex: codexLogo,
  copilot: copilotLogo,
  "github-copilot": copilotLogo,
  cursor: cursorLogo,
  deepseek: deepseekLogo,
  dsh: deepseekLogo,
  "deepseek-harness": deepseekLogo,
  kimi: kimiLogo,
  "kimi-code": kimiLogo,
  kilo: kiloLogo,
  "kilo-cli": kiloLogo,
  kilocode: kiloLogo,
  openclaw: openclawLogo,
  qoder: qoderLogo,
  qodercn: qoderLogo,
  "qoder-cn": qoderLogo,
  roo: rooLogo,
  "roo-code": rooLogo,
  workbuddy: workbuddyLogo,
};

function logoFor(id: string, name?: string) {
  const values = [id, name ?? ""].map((value) => value.trim().toLowerCase());
  return values.map((value) => logos[value]).find(Boolean);
}

export function getAgentBrandLogo(id: string, name?: string) {
  return logoFor(id, name);
}

export function AgentBrandIcon({ id, name, color, className }: { id: string; name?: string; color?: string; className?: string }) {
  const logo = logoFor(id, name);
  if (logo) return <img className={className} src={logo} alt="" aria-hidden="true" />;
  return <span className={className} style={{ color }}>{(name || id).slice(0, 1).toUpperCase()}</span>;
}
