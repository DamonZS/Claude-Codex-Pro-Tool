import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { stageMulticaNotices, verifyMulticaNotices } from "./stage-multica-notices.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const source = join(root, "docs/third-party/multica");
const script = fileURLToPath(new URL("./stage-multica-notices.mjs", import.meta.url));

function fixture(t) {
  const directory = mkdtempSync(join(root, "scripts/release/.notices-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  return join(directory, "resources with spaces", "third-party", "multica");
}

test("stages only the complete LICENSE and NOTICE and permits repeat staging", (t) => {
  const destination = fixture(t);
  stageMulticaNotices(destination);
  stageMulticaNotices(destination);
  assert.deepEqual(readdirSync(destination).sort(), ["LICENSE", "NOTICE"]);
  for (const file of ["LICENSE", "NOTICE"]) {
    assert.deepEqual(readFileSync(join(destination, file)), readFileSync(join(source, file)));
  }
  verifyMulticaNotices(destination);
});

test("verification rejects a missing notice", (t) => {
  const destination = fixture(t);
  stageMulticaNotices(destination);
  rmSync(join(destination, "NOTICE"));
  assert.throws(() => verifyMulticaNotices(destination), /ENOENT/);
});

test("verification rejects truncated or changed license text", (t) => {
  const destination = fixture(t);
  stageMulticaNotices(destination);
  writeFileSync(join(destination, "LICENSE"), "Multica\n");
  assert.throws(() => verifyMulticaNotices(destination), /LICENSE differs from source/);
});

test("CLI resolves source independently of cwd and verifies without rewriting", (t) => {
  const destination = fixture(t);
  const options = { cwd: dirname(script), encoding: "utf8" };
  const staged = spawnSync(process.execPath, [script, destination], options);
  assert.equal(staged.status, 0, staged.stderr);
  const verified = spawnSync(process.execPath, [script, destination, "--verify"], options);
  assert.equal(verified.status, 0, verified.stderr);
  writeFileSync(join(destination, "NOTICE"), "changed");
  const changed = spawnSync(process.execPath, [script, destination, "--verify"], options);
  assert.notEqual(changed.status, 0);
  assert.equal(readFileSync(join(destination, "NOTICE"), "utf8"), "changed");
});

test("CLI rejects missing destination or unknown mode", (t) => {
  const destination = fixture(t);
  for (const args of [[], [destination, "--unknown"]]) {
    const result = spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Usage:/);
  }
});
