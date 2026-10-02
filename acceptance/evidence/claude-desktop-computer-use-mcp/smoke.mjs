// Smoke test for `claude-codex-pro --mcp-computer-use` over stdio.
// Usage: node acceptance/evidence/claude-desktop-computer-use-mcp/smoke.mjs <path-to-claude-codex-pro.exe>
// Does not change settings; with the CCP toggle off, `screenshot` must return isError.
import { spawn } from "node:child_process";
import readline from "node:readline";

const exe = process.argv[2];
if (!exe) {
  console.error("usage: computer-use-mcp-smoke.mjs <exe>");
  process.exit(2);
}

const child = spawn(exe, ["--mcp-computer-use"], { stdio: ["pipe", "pipe", "inherit"] });
const lines = readline.createInterface({ input: child.stdout });
const pending = new Map();
lines.on("line", (line) => {
  const message = JSON.parse(line);
  pending.get(message.id)?.(message);
  pending.delete(message.id);
});

let nextId = 1;
function request(method, params = {}) {
  const id = nextId++;
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
  return new Promise((resolve, reject) => {
    pending.set(id, resolve);
    setTimeout(() => reject(new Error(`timeout: ${method}`)), 15000);
  });
}

function check(label, condition) {
  console.log(`${condition ? "PASS" : "FAIL"} ${label}`);
  if (!condition) process.exitCode = 1;
}

try {
  const init = await request("initialize", { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "smoke", version: "0" } });
  check("initialize protocolVersion", init.result?.protocolVersion === "2025-06-18");
  check("initialize serverInfo", init.result?.serverInfo?.name === "claude-codex-pro-computer-use");
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })}\n`);

  const list = await request("tools/list");
  const names = (list.result?.tools ?? []).map((tool) => tool.name);
  check(`tools/list (${names.join(",")})`, names.length === 9 && names.includes("screenshot"));

  const unknown = await request("nope");
  check("unknown method -32601", unknown.error?.code === -32601);

  const shot = await request("tools/call", { name: "screenshot", arguments: {} });
  const text = shot.result?.content?.[0]?.text ?? shot.result?.content?.[0]?.type;
  console.log(`screenshot isError=${shot.result?.isError} first=${String(text).slice(0, 80)}`);
  check("screenshot gated or returns jpeg", shot.result?.isError === true || shot.result?.content?.[0]?.mimeType === "image/jpeg");
} catch (error) {
  console.log(`FAIL ${error.message}`);
  process.exitCode = 1;
} finally {
  child.stdin.end();
  child.kill();
}
