// @vitest-environment node
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import ts from "typescript";

const root = fileURLToPath(new URL("../", import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(root, "vendor/multica/source-files.json"), "utf8")) as {
  revision: string;
  files: Array<{ source: string; destination: string; sha256: string; derivedSha256: string; changes: string[] }>;
};
describe("pinned original source provenance", () => {
  it("verifies every copied and modified file against its recorded SHA-256", () => {
    expect(manifest.revision).toBe("9fce92f427694d7d303258aa281b05c902a95ba9");
    for (const entry of manifest.files) {
      const actual = createHash("sha256").update(fs.readFileSync(path.join(root, entry.destination))).digest("hex");
      expect(actual, entry.source).toBe(entry.derivedSha256);
      if (!entry.changes.length) expect(actual, entry.source).toBe(entry.sha256);
    }
  });
  it("preserves the three actual upstream page bodies unchanged", () => {
    for (const page of ["my-issues/components/my-issues-page.tsx", "autopilots/components/autopilots-page.tsx", "agents/components/agents-page.tsx"]) {
      const entry = manifest.files.find((entry) => entry.source === `packages/views/${page}`);
      expect(entry).toBeDefined();
      expect(entry?.changes).toEqual([]);
      expect(entry?.sha256).toBe(entry?.derivedSha256);
    }
  });
  it("retains full license texts and preserves source bytes across Git checkouts", () => {
    for (const name of ["LICENSE", "NOTICE"]) {
      const vendor = fs.readFileSync(path.join(root, "vendor/multica", name), "utf8");
      const packaged = fs.readFileSync(path.join(root, "../../docs/third-party/multica", name), "utf8");
      expect(packaged.replaceAll("\r\n", "\n")).toBe(vendor);
    }
    expect(fs.readFileSync(path.join(root, "vendor/.gitattributes"), "utf8")).toContain("* -text");
  });
  it("contains no ambient fetch or socket construction in the copied source", () => {
    const violations: string[] = [];
    for (const entry of manifest.files.filter((entry) => /\.[jt]sx?$/.test(entry.source))) {
      const source = fs.readFileSync(path.join(root, entry.destination), "utf8");
      const ast = ts.createSourceFile(entry.source, source, ts.ScriptTarget.Latest, true);
      function walk(node: ts.Node) {
        if (ts.isCallExpression(node) && ["fetch", "window.fetch", "globalThis.fetch"].includes(node.expression.getText(ast))) violations.push(entry.source);
        if (ts.isNewExpression(node) && ["WebSocket", "EventSource", "XMLHttpRequest"].includes(node.expression.getText(ast))) violations.push(entry.source);
        ts.forEachChild(node, walk);
      }
      walk(ast);
    }
    expect(violations).toEqual([]);
  });
  it("scopes original token and dark styles across the shadow boundary", () => {
    const tokens = fs.readFileSync(path.join(root, "vendor/multica/packages/ui/styles/tokens.css"), "utf8");
    expect(tokens).not.toContain(":root");
    expect(tokens).toContain(":host, .ccp-workflow-surface");
    expect(tokens).toContain(".dark .ccp-workflow-surface");
  });
});
