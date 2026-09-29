const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const here = __dirname;
const root = path.resolve(here, "../..");
const baseline = path.join(here, "baseline");
const modified = path.join(here, "MODIFIED_FILE");
const restored = path.join(here, "rollback-copy");
const reconstructed = path.join(here, "patch-copy");
const ledger = [];
function files(directory, prefix = "") {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const relative = `${prefix}${entry.name}`;
    return entry.isDirectory() ? files(path.join(directory, entry.name), `${relative}/`) : [relative];
  });
}
function hash(file) { return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex"); }
function run(label, command, args, cwd = root) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 });
  ledger.push({ label, command: [command, ...args], cwd, exit: result.status, stdout: result.stdout ?? "", stderr: result.stderr ?? "", error: result.error?.message });
  console.log(`${label}: exit=${result.status}`);
  return result.status;
}
const originals = files(baseline);
const additions = ["apps/claude-codex-pro-manager/src/lib/overviewUsage.ts", "scripts/test-overview-usage.cjs"];
for (const relative of [...originals, ...additions]) {
  const target = path.join(modified, relative);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(path.join(root, relative), target);
}
const diff = spawnSync("git", ["-c", "core.autocrlf=false", "diff", "--no-index", "--binary", "--", "baseline", "MODIFIED_FILE"], { cwd: here, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 });
if (diff.status !== 1) throw new Error(`diff failed: ${diff.status}: ${diff.stderr}`);
const patch = diff.stdout.replaceAll("a/baseline/", "a/").replaceAll("b/MODIFIED_FILE/", "b/").replaceAll("a/MODIFIED_FILE/", "a/").replaceAll("b/baseline/", "b/");
fs.writeFileSync(path.join(here, "DIFF_FILE.patch"), patch);
const testScript = path.join(root, "scripts/test-overview-usage.cjs");
run("BASELINE", process.execPath, [testScript, baseline]);
run("MODIFIED", process.execPath, [testScript, modified]);
fs.cpSync(modified, restored, { recursive: true });
fs.writeFileSync(path.join(restored, ".ccp-transaction-copy"), "disposable verification copy\n");
run("ROLLBACK_COMMAND", "H:/git/bin/bash.exe", [path.join(here, "ROLLBACK.sh").replaceAll("\\", "/"), restored.replaceAll("\\", "/")]);
run("ROLLBACK", process.execPath, [testScript, restored]);
const restoredHashes = originals.map((relative) => ({ relative, baseline: hash(path.join(baseline, relative)), restored: hash(path.join(restored, relative)), modified: hash(path.join(modified, relative)) }));
if (restoredHashes.some((item) => item.baseline !== item.restored)) throw new Error("rollback hash mismatch");
fs.cpSync(baseline, reconstructed, { recursive: true });
const patchExit = run("REAPPLY_PATCH", "git", ["-c", "core.autocrlf=false", "apply", `--directory=${path.relative(root, reconstructed).replaceAll("\\", "/")}`, "--", path.join(here, "DIFF_FILE.patch")]);
const reapplied = [...originals, ...additions].map((relative) => ({ relative, expected: hash(path.join(modified, relative)), actual: fs.existsSync(path.join(reconstructed, relative)) ? hash(path.join(reconstructed, relative)) : null }));
const patchMatches = patchExit === 0 && reapplied.every((item) => item.actual === item.expected);
ledger.push({ label: "SHA256", restoredHashes, reapplied, patchMatches });
fs.writeFileSync(path.join(here, "VERIFICATION.txt"), ledger.map((entry) => JSON.stringify(entry, null, 2)).join("\n\n") + "\n");
if (!patchMatches) throw new Error("reconstructed patch hash mismatch; see VERIFICATION.txt");
if (ledger.find((entry) => entry.label === "MODIFIED").exit !== 0) throw new Error("modified verification failed");
for (const role of ["MODIFIED_FILE", "DIFF_FILE.patch", "VERIFICATION.txt", "ROLLBACK.sh"]) {
  const target = path.join(here, role);
  if (fs.statSync(target).isDirectory()) for (const relative of files(target)) fs.readFileSync(path.join(target, relative));
  else fs.readFileSync(target);
  console.log(`REOPENED: ${target}`);
}
