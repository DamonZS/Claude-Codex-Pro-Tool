#!/usr/bin/env node
import assert from "node:assert/strict";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const source = fileURLToPath(new URL("../../docs/third-party/multica/", import.meta.url));
const files = ["LICENSE", "NOTICE"];

export function verifyMulticaNotices(destination) {
  for (const file of files) {
    const original = readFileSync(join(source, file));
    assert.ok(original.length > 0, `Multica ${file} is empty`);
    assert.ok(readFileSync(join(destination, file)).equals(original), `Multica ${file} differs from source`);
  }
}

export function stageMulticaNotices(destination) {
  // Validate both inputs before staging any files.
  for (const file of files) {
    assert.ok(readFileSync(join(source, file)).length > 0, `Multica ${file} is empty`);
  }
  mkdirSync(destination, { recursive: true });
  for (const file of files) copyFileSync(join(source, file), join(destination, file));
  verifyMulticaNotices(destination);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [destination, mode] = process.argv.slice(2);
  assert.ok(destination && (!mode || mode === "--verify") && process.argv.length <= 4,
    "Usage: node stage-multica-notices.mjs DESTINATION [--verify]");
  if (mode === "--verify") verifyMulticaNotices(destination);
  else stageMulticaNotices(destination);
  console.log("Multica LICENSE and NOTICE verified byte-for-byte");
}
