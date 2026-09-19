import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import postcss from "postcss";
import { adaptWebhookUi } from "./webhook-ui-patches.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const repo = path.resolve(root, "../..");
const source = path.resolve(process.argv[2] || path.join(repo, ".upstream-multica"));
const revision = "9fce92f427694d7d303258aa281b05c902a95ba9";
const destination = path.join(root, "vendor/multica");
const git = (...args) => execFileSync("git", ["-C", source, ...args], { maxBuffer: 128 * 1024 * 1024 });
const files = new Set(git("ls-tree", "-r", "--name-only", revision).toString().trim().split("\n"));
const seeds = [
  "packages/views/my-issues/components/my-issues-page.tsx",
  "packages/views/settings/components/properties-tab.tsx",
  "packages/views/autopilots/components/autopilots-page.tsx",
  "packages/views/autopilots/components/autopilot-detail-page.tsx",
  "packages/views/agents/components/agents-page.tsx",
  "packages/views/agents/components/agent-detail-page.tsx",
  "packages/views/agents/create/manual-create-agent-page.tsx",
  "packages/views/agents/create/choose-create-method-page.tsx",
  "packages/views/agents/create/ai-create-agent-page.tsx",
  "packages/views/agents/create/ai-builder-session-page.tsx",
  "packages/views/issues/components/issue-detail-route.tsx",
  "packages/views/modals/registry.tsx",
  "packages/views/navigation/index.ts",
  "packages/views/i18n/resources-types.ts",
  "packages/core/provider.tsx",
  "packages/core/auth/index.ts",
  "packages/core/i18n/provider.tsx",
  "packages/core/paths/hooks.tsx",
  "packages/core/realtime/provider.tsx",
  "packages/ui/styles/tokens.css",
  "packages/ui/styles/base.css",
  "LICENSE", "NOTICE",
];
// Read pinned Git objects, never mutable worktree contents.
const paths = [...files].filter((p) => p.startsWith("packages/") || seeds.includes(p));
const batch = execFileSync("git", ["-C", source, "cat-file", "--batch"], {
  input: paths.map((p) => `${revision}:${p}\n`).join(""), maxBuffer: 128 * 1024 * 1024,
});
let offset = 0;
const blobs = new Map(paths.map((p) => {
  const end = batch.indexOf(10, offset);
  const size = Number(batch.subarray(offset, end).toString().split(" ")[2]);
  const data = batch.subarray(end + 1, end + 1 + size);
  offset = end + size + 2;
  return [p, data];
}));
const packageExports = new Map(["core", "ui", "views"].map((name) => [name,
  JSON.parse(blobs.get(`packages/${name}/package.json`).toString()).exports,
]));
function resolve(specifier, importer) {
  let candidate;
  if (specifier.startsWith(".")) candidate = path.posix.normalize(path.posix.join(path.posix.dirname(importer), specifier));
  else if (specifier.startsWith("@multica/")) {
    const [, name, rest = ""] = /^@multica\/([^/]+)\/?(.*)$/.exec(specifier);
    const exports = packageExports.get(name);
    let target = exports?.[rest ? `./${rest}` : "."];
    if (!target && exports) {
      for (const [key, value] of Object.entries(exports)) {
        if (!key.endsWith("*") || !`./${rest}`.startsWith(key.slice(0, -1))) continue;
        target = typeof value === "string" ? value.replace("*", `./${rest}`.slice(key.length - 1)) : undefined;
        if (target) break;
      }
    }
    candidate = `packages/${name}/${typeof target === "string" ? target.replace(/^\.\//, "") : rest}`;
  } else return null;
  candidate = candidate.replace(/\?(raw|inline|url)$/, "");
  for (const name of [candidate, ...[".ts", ".tsx", ".json", ".js", "/index.ts", "/index.tsx"].map((ext) => candidate + ext)]) {
    if (files.has(name)) return name;
  }
  throw new Error(`Unresolved ${specifier} from ${importer}`);
}
// Locale type augmentation references English; ship matching Chinese resources too.
for (const p of files) if (/^packages\/views\/locales\/(en|zh-Hans)\/.*\.json$/.test(p)) seeds.push(p);
const selected = new Set();
const queue = [...seeds];
while (queue.length) {
  const name = queue.pop();
  if (selected.has(name)) continue;
  const data = blobs.get(name);
  if (!data) throw new Error(`Missing upstream file: ${name}`);
  selected.add(name);
  if (/\.[cm]?[jt]sx?$/.test(name)) {
    const info = ts.preProcessFile(data.toString(), true, true);
    for (const imported of info.importedFiles) {
      const dependency = resolve(imported.fileName, name);
      if (dependency) queue.push(dependency);
    }
  }
  if (name.endsWith(".css")) {
    postcss.parse(data.toString()).walkAtRules("import", (rule) => {
      const specifier = rule.params.match(/^["']([^"']+)["']/)?.[1];
      if (specifier) { const dependency = resolve(specifier, name); if (dependency) queue.push(dependency); }
    });
  }
}
const sha = (data) => createHash("sha256").update(data).digest("hex");
function adapt(name, data) {
  let text = data.toString();
  const changes = [];
  const imports = new Set();
  // Unified executions stay in upstream cards, rows and tables. Keep every
  // derived change here so a fresh vendor run preserves the readonly contract.
  const executionImports = new Set();
  const nativeImport = (symbols, module) => {
    const relative = path.relative(path.dirname(path.join(destination, name)), path.join(root, `src/${module}`)).split(path.sep).join("/");
    text = `import { ${symbols} } from "${relative}";\n${text}`;
  };
  const executionPatch = (before, after, ...symbols) => {
    if (!text.includes(before)) throw new Error(`Execution UI source changed: ${name}: ${before}`);
    text = text.replaceAll(before, after);
    symbols.forEach((symbol) => executionImports.add(symbol));
    if (!changes.includes("Project execution metadata and protect readonly virtual issues")) changes.push("Project execution metadata and protect readonly virtual issues");
  };
  if (name === "packages/views/issues/surface/selection-context.tsx") {
    executionPatch('const toggle = useCallback((id: string) => {', 'const toggle = useCallback((id: string) => {\n    if (isExecutionIssueId(id)) return;', "isExecutionIssueId");
    executionPatch('for (const id of ids) next.add(id);', 'for (const id of ids) if (!isExecutionIssueId(id)) next.add(id);');
  }
  if (name === "packages/views/issues/utils/drag-utils.ts") {
    executionPatch('  const index = ids.indexOf(activeId);\n  return {\n    before_id:', '  ids = ids.filter((id) => !isExecutionIssueId(id));\n  const index = ids.indexOf(activeId);\n  return {\n    before_id:', "isExecutionIssueId");
  }
  if (name === "packages/views/issues/components/board-card.tsx") {
    nativeImport("isNativeExecutionCard", "native-execution-actions");
    executionPatch('  disableSorting,', '  disableSorting,\n  nativeExecutionDrag = false,');
    executionPatch('  disableSorting?: boolean;', '  disableSorting?: boolean;\n  nativeExecutionDrag?: boolean;');
    executionPatch('const canEdit = editable && !!surfaceActions;', 'const canEdit = editable && !!surfaceActions && !isReadonlyExecutionIssue(issue);', "isReadonlyExecutionIssue");
    executionPatch('disabled: disableSorting ? { droppable: true } : undefined,', 'disabled: isReadonlyExecutionIssue(issue) && !(nativeExecutionDrag && isNativeExecutionCard(issue)) ? true : disableSorting ? { droppable: true } : undefined,');
    executionPatch('data-board-card=""', 'data-board-card=""\n        data-ccp-issue-id={issue.id}\n        data-ccp-execution-state={issue.metadata?.ccp_execution_state}\n        data-ccp-source={issue.metadata?.ccp_source}');
    executionPatch('{issue.title}\n      </p>', '{issue.title}\n      </p>\n      <ExecutionBadge issue={issue} />', "ExecutionBadge");
    executionPatch('? getActorName(issue.assignee_type, issue.assignee_id)', '? String(issue.metadata?.ccp_agent_name ?? getActorName(issue.assignee_type, issue.assignee_id))');
    executionPatch('        enableHoverCard', '        enableHoverCard={!isReadonlyExecutionIssue(issue)}');
  }
  if (name === "packages/views/issues/components/board-view.tsx") {
    nativeImport("useNativeExecutionActions", "native-execution-actions");
    nativeImport("NativeExecutionFeedback", "native-execution-feedback");
    executionPatch('  const boardWsId = useWorkspaceId();', '  const boardWsId = useWorkspaceId();\n  const submitNativeIntent = useNativeExecutionActions();');
    executionPatch('      if (!over || recentlyMovedRef.current) return;', '      if (!over || recentlyMovedRef.current) return;\n      const dragged = issueMapRef.current.get(active.id as string);\n      if (dragged && isReadonlyExecutionIssue(dragged)) return;', "isReadonlyExecutionIssue");
    executionPatch('      // Same-column reorder (manual sort only)', `      const dragged = issueMapRef.current.get(activeId);
      if (dragged && isReadonlyExecutionIssue(dragged)) {
        resetColumns();
        const target = groupMap.get(overCol);
        if (target?.status && !issueMatchesGroup(dragged, target)) void submitNativeIntent(dragged, target.status);
        return;
      }

      // Same-column reorder (manual sort only)`);
    executionPatch('onMoveIssue, groupIds, groupMap, sortBy, beginSettle', 'onMoveIssue, submitNativeIntent, groupIds, groupMap, sortBy, beginSettle');
    executionPatch('      <div\n        ref={pan.ref}', '      <NativeExecutionFeedback issues={groupedIssues} />\n      <div\n        ref={pan.ref}');
  }
  if (name === "packages/views/issues/components/board-column.tsx") {
    executionPatch('ref={mergedRef}', 'ref={mergedRef}\n          data-ccp-status-category={group.status}');
    executionPatch('disableSorting={!!sortLabel}', 'disableSorting={!!sortLabel}\n        nativeExecutionDrag={group.status !== undefined}');
  }
  if (name === "packages/views/issues/components/list-row.tsx") {
    executionPatch('disabled: disableSorting ? { droppable: true } : undefined,', 'disabled: isReadonlyExecutionIssue(issue) ? true : disableSorting ? { droppable: true } : undefined,', "isReadonlyExecutionIssue");
    executionPatch('ref={containerRef}', 'ref={containerRef}\n        data-ccp-issue-id={issue.id}\n        data-ccp-execution-state={issue.metadata?.ccp_execution_state}\n        data-ccp-source={issue.metadata?.ccp_source}');
    executionPatch('type="checkbox"', 'type="checkbox"\n            disabled={isReadonlyExecutionIssue(issue)}');
    executionPatch('<span className="truncate">{issue.title}</span>', '<span className="truncate">{issue.title}</span>\n            <ExecutionBadge issue={issue} />', "ExecutionBadge");
    executionPatch('              enableHoverCard', '              enableHoverCard={!isReadonlyExecutionIssue(issue)}\n              profileLink={!isReadonlyExecutionIssue(issue)}');
  }
  if (name === "packages/views/issues/components/list-view.tsx") {
    executionPatch('const allSelected = issues.length > 0 && selectedCount === issues.length;', 'const selectableCount = issues.filter((issue) => !isReadonlyExecutionIssue(issue)).length;\n  const allSelected = selectableCount > 0 && selectedCount === selectableCount;', "isReadonlyExecutionIssue");
    executionPatch('checked={allSelected}', 'checked={allSelected}\n            disabled={selectableCount === 0}');
  }
  if (name === "packages/views/issues/actions/issue-actions-context-menu.tsx") {
    executionPatch('    event.preventDefault();', '    event.preventDefault();\n    if (isReadonlyExecutionIssue(issue)) return;', "isReadonlyExecutionIssue");
  }
  if (name === "packages/views/issues/actions/issue-actions-dropdown.tsx") {
    executionPatch('  const [assigneeOpen, setAssigneeOpen] = useState(false);', '  const [assigneeOpen, setAssigneeOpen] = useState(false);\n  if (isReadonlyExecutionIssue(issue)) return null;', "isReadonlyExecutionIssue");
  }
  if (name === "packages/views/issues/components/batch-action-toolbar.tsx") {
    executionPatch('issues.filter((i) => selectedIds.has(i.id))', 'issues.filter((i) => selectedIds.has(i.id) && !isReadonlyExecutionIssue(i))', "isReadonlyExecutionIssue");
  }
  if (name === "packages/views/issues/surface/use-issue-surface-actions.ts") {
    executionPatch('      updateIssueMutation.mutate(', '      if (isExecutionIssueId(issueId)) { options?.onSettled?.(); return; }\n      updateIssueMutation.mutate(', "isExecutionIssueId");
    // moveIssue has its own mutation and only an onSettled callback.
    text = text.replace('const { before_id, after_id, ...optimisticUpdates } = updates;\n      if (isExecutionIssueId(issueId)) { options?.onSettled?.(); return; }', 'const { before_id, after_id, ...optimisticUpdates } = updates;\n      if (isExecutionIssueId(issueId)) { onSettled?.(); return; }');
    executionPatch('        await batchUpdateMutation.mutateAsync({ ids: issueIds, updates });', '        const ids = issueIds.filter((id) => !isExecutionIssueId(id));\n        if (ids.length) await batchUpdateMutation.mutateAsync({ ids, updates });');
    executionPatch('        await batchDeleteMutation.mutateAsync(issueIds);', '        const ids = issueIds.filter((id) => !isExecutionIssueId(id));\n        if (ids.length) await batchDeleteMutation.mutateAsync(ids);');
  }
  if (name === "packages/views/issues/components/table-view.tsx") {
    executionPatch('const checked = issueIds.length > 0 && selectedCount === issueIds.length;', 'const selectableIds = issueIds.filter((id) => !isExecutionIssueId(id));\n  const checked = selectableIds.length > 0 && selectedCount === selectableIds.length;', "isExecutionIssueId");
    executionPatch('  const issue = row.original.issue;\n  return (', '  const issue = row.original.issue;\n  if (isReadonlyExecutionIssue(issue)) return null;\n  return (', "isReadonlyExecutionIssue");
    executionPatch('  const commit = () => {\n    const title = draft.trim();', `  if (isReadonlyExecutionIssue(row.issue)) return <div className="flex min-w-0 items-center gap-1.5" data-ccp-issue-id={row.issue.id}>
    <span className="text-caption text-muted-foreground">{row.issue.identifier}</span>
    <button type="button" className="truncate text-left hover:underline" onClick={(event) => { event.stopPropagation(); onOpen(event); }}>{row.issue.title}</button>
    <ExecutionBadge issue={row.issue} />
  </div>;
  const commit = () => {
    const title = draft.trim();`, "ExecutionBadge");
    executionPatch('            {row.issue.title}\n          </button>', '            {row.issue.title}\n          </button>\n          <ExecutionBadge issue={row.issue} />');
    executionPatch('  const propertyId = propertyIdFromViewKey(key);\n  if (propertyId) {', `  const propertyId = propertyIdFromViewKey(key);
  if (isReadonlyExecutionIssue(issue) && key !== "title") {
    if (propertyId) return <span>{String(issue.properties?.[propertyId] ?? "—")}</span>;
    if (key === "assignee") return <span>{String(issue.metadata?.ccp_agent_name ?? "—")}</span>;
    if (key === "status") return <ExecutionBadge issue={issue} />;
    if (key === "labels") return <span>{issue.labels?.map(label => label.name).join(", ") || "—"}</span>;
    return <span>{String(issue[key as keyof Issue] ?? "—")}</span>;
  }
  if (propertyId) {`);
  }
  if (executionImports.size) {
    const relative = path.relative(path.dirname(path.join(destination, name)), path.join(root, "src/execution-issue")).split(path.sep).join("/");
    text = `import { ${[...executionImports].join(", ")} } from "${relative}";\n${text}`;
  }
  if (/\.[jt]sx?$/.test(name)) {
    const ast = ts.createSourceFile(name, text, ts.ScriptTarget.Latest, true);
    const edits = [];
    function walk(node) {
      if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "fetch") {
        edits.push([node.expression.getStart(ast), node.expression.end, "workflowFetch"]);
        imports.add("workflowFetch");
      }
      if (ts.isNewExpression(node) && ts.isIdentifier(node.expression) && ["WebSocket", "EventSource"].includes(node.expression.text)) {
        edits.push([node.getStart(ast), node.end, "blockedSocket()"]);
        imports.add("blockedSocket");
      }
      if ((ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) && node.tagName.getText(ast).endsWith(".Portal")) {
        edits.push([node.tagName.end, node.tagName.end, " container={workflowPortal()}"]);
        imports.add("workflowPortal");
      }
      if (ts.isCallExpression(node) && node.expression.getText(ast) === "createPortal" && node.arguments.length >= 2) {
        const target = node.arguments[1];
        if (target.getText(ast) === "document.body") {
          edits.push([target.getStart(ast), target.end, "workflowPortal()"]);
          imports.add("workflowPortal");
        }
      }
      if (ts.isPropertyAccessExpression(node) && node.name.text === "body" && ["doc", "document"].includes(node.expression.getText(ast)) &&
          ts.isPropertyAccessExpression(node.parent) && ["appendChild", "removeChild"].includes(node.parent.name.text)) {
        edits.push([node.getStart(ast), node.end, "workflowPortal()"]);
        imports.add("workflowPortal");
      }
      ts.forEachChild(node, walk);
    }
    walk(ast);
    for (const [start, end, value] of edits.sort((a, b) => b[0] - a[0])) text = text.slice(0, start) + value + text.slice(end);
    if (edits.length) changes.push("Route network and/or portals through the CCP host boundary");
  }
  if (name === "packages/core/realtime/provider.tsx") {
    text = text.replace("const WSContext =", "export const WSContext =");
    changes.push("Export realtime context for the local event provider; no WSProvider mounted");
  }
  if (name === "packages/views/settings/components/properties-tab.tsx") {
    const gate = '  const canManage = currentMember?.role === "owner" || currentMember?.role === "admin";';
    if (!text.includes(gate)) throw new Error("Property management gate source changed");
    text = text.replace('export function PropertiesTab() {', 'export function PropertiesTab({ localCanManage, managementHint }: { localCanManage?: boolean; managementHint?: string } = {}) {')
      .replace(gate, '  const canManage = localCanManage ?? (currentMember?.role === "owner" || currentMember?.role === "admin");')
      .replace('const { data: properties = [], isLoading }', 'const { data: properties = [], isLoading, isError, refetch }')
      .replace(') : visible.length === 0 ? (', `) : isError ? (
            <div role="alert" className="px-4 py-12 text-center">
              <p>{t(($) => $.properties.load_failed)}</p>
              <Button variant="outline" onClick={() => void refetch()}>{t(($) => $.properties.retry)}</Button>
            </div>
          ) : visible.length === 0 ? (`)
      .replaceAll('t(($) => $.properties.editor.admin_hint)', 'managementHint ?? t(($) => $.properties.editor.admin_hint)')
      .replace('<PropertyEditorDialog open={createOpen}', '<PropertyEditorDialog managementHint={managementHint} open={createOpen}')
      .replace('        property={editing}', '        managementHint={managementHint}\n        property={editing}')
      .replace('function PropertyEditorDialog({\n  open,', 'function PropertyEditorDialog({\n  managementHint,\n  open,')
      .replace('  property?: IssueProperty | null;', '  property?: IssueProperty | null;\n  managementHint?: string;');
    changes.push("Accept authoritative local catalog capability without changing membership; expose query errors and retry");
  }
  if (/^packages\/views\/locales\/(en|zh-Hans)\/settings\.json$/.test(name)) {
    const locale = JSON.parse(text);
    Object.assign(locale.properties, name.includes('/en/')
      ? { load_failed: "Failed to load properties", retry: "Retry" }
      : { load_failed: "属性加载失败", retry: "重试" });
    text = JSON.stringify(locale, null, 2) + "\n";
    changes.push("Add property catalog query error and retry translations");
  }
  if (name === "packages/core/runtimes/cli-version.ts") {
    const marker = "export function readRuntimeCliVersion(metadata: Record<string, unknown> | undefined): string {\n  const v = metadata?.cli_version;\n  return typeof v === \"string\" ? v : \"\";\n}";
    if (!text.includes(marker)) throw new Error("Quick-create runtime gate source changed");
    text = text.replace(marker, marker + `

/** Native task execution has no Multica daemon; all other runtimes retain the CLI gate. */
export function quickCreateVersionBlocked(
  metadata: Record<string, unknown> | undefined,
  usesExplicitFields: boolean,
): boolean {
  if (metadata?.nativeTaskHostSupported === true) return false;
  const version = readRuntimeCliVersion(metadata);
  return checkQuickCreateCliVersion(version).state !== "ok" ||
    (usesExplicitFields && checkQuickCreateFieldsCliVersion(version).state !== "ok");
}`);
    changes.push("Gate quick create on native task-host capability without a synthetic CLI version");
  }
  if (name === "packages/core/runtimes/models.ts") {
    const marker = "  const initial = await api.initiateListModels(runtimeId);";
    if (!text.includes(marker)) throw new Error("Runtime model discovery source changed");
    text = text.replace(marker, `  const runtime = (await api.listRuntimes()).find((entry) => entry.id === runtimeId);
  if (runtime?.metadata?.nativeModelSelectionAuthoritative === true) {
    return { models: [], unavailableModels: [], supported: false, cached: false };
  }
${marker}`);
    changes.push("Keep native model selection authoritative without daemon discovery");
  }
  if (name === "packages/views/modals/quick-create-issue.tsx") {
    const marker = '  const versionBlocked =\n    baseVersionCheck.state !== "ok" ||\n    (usesExplicitFields && fieldVersionCheck.state !== "ok");';
    if (!text.includes(marker)) throw new Error("Quick-create modal gate source changed");
    text = text.replace("  readRuntimeCliVersion,", "  readRuntimeCliVersion,\n  quickCreateVersionBlocked,");
    text = text.replace(marker, "  const versionBlocked = quickCreateVersionBlocked(selectedRuntime?.metadata, usesExplicitFields);");
    changes.push("Gate quick create on native task-host capability without a synthetic CLI version");
  }
  if (name === "packages/views/issues/components/issue-detail-route.tsx") {
    text = text.replace('import { useEffect, useRef, useState } from "react";', 'import { useEffect, useRef, useState, type ReactNode } from "react";')
      .replace('  onDelete?: () => void;', '  onDelete?: () => void;\n  leadingAction?: ReactNode;')
      .replace('IssueDetailRoute({ routeId, onDelete }', 'IssueDetailRoute({ routeId, onDelete, leadingAction }')
      .replace('<IssueDetailSkeleton />', '<IssueDetailSkeleton leading={leadingAction} />')
      .replace('<IssueNotFound showBackLink={!onDelete} />', '<IssueNotFound showBackLink={!onDelete && !leadingAction} leading={leadingAction} />')
      .replace('      issueId={canonicalId}', '      leadingAction={leadingAction}\n      issueId={canonicalId}');
    if (!text.includes('leading={leadingAction}') || !text.includes('leadingAction={leadingAction}')) throw new Error("Issue detail leading slot source changed");
    changes.push("Forward the embedded host return control through detail, loading and not-found states");
  }
  if (name === "packages/views/autopilots/components/autopilot-detail-page.tsx") {
    text = text.replace('import { useState } from "react";', 'import { useState, type ReactNode } from "react";')
      .replace('export function AutopilotDetailPage({ autopilotId }: { autopilotId: string }) {', 'export function AutopilotDetailPage({ autopilotId, leadingAction }: { autopilotId: string; leadingAction?: ReactNode }) {')
      .replace('<PageHeader>\n          <Skeleton className="h-4 w-4" />', '<PageHeader leading={leadingAction}>\n          <Skeleton className="h-4 w-4" />')
      .replace('    return (\n      <div className="flex items-center justify-center h-full text-muted-foreground">\n        {t(($) => $.detail.not_found)}\n      </div>\n    );', '    return (\n      <div className="flex h-full flex-col">\n        <PageHeader leading={leadingAction}>\n          <AppLink href={wsPaths.autopilots()} className="text-muted-foreground hover:text-foreground">\n            {t(($) => $.page.title)}\n          </AppLink>\n        </PageHeader>\n        <div className="flex flex-1 items-center justify-center text-muted-foreground">\n          {t(($) => $.detail.not_found)}\n        </div>\n      </div>\n    );')
      .replace('<BreadcrumbHeader\n        segments=', '<BreadcrumbHeader\n        leading={leadingAction}\n        segments=');
    if (!text.includes('leading={leadingAction}') || !text.includes('leadingAction?: ReactNode')) throw new Error("Autopilot detail leading slot source changed");
    changes.push("Expose a host return control for autopilot detail, loading and not-found states");
  }
  if (name === "packages/views/agents/components/agent-detail-page.tsx") {
    text = text.replace('import { useState } from "react";', 'import { useState, type ReactNode } from "react";')
      .replace('  agentId: string;\n}', '  agentId: string;\n  leadingAction?: ReactNode;\n}')
      .replace('export function AgentDetailPage({ agentId }: AgentDetailPageProps) {', 'export function AgentDetailPage({ agentId, leadingAction }: AgentDetailPageProps) {')
      .replace('return <DetailLoadingSkeleton />;', 'return <DetailLoadingSkeleton leading={leadingAction} />;')
      .replace('        onArchive={\n          agent.system_key ? undefined : () => setConfirmArchive(true)\n        }\n      />', '        onArchive={\n          agent.system_key ? undefined : () => setConfirmArchive(true)\n        }\n        leadingAction={leadingAction}\n      />')
      .replace('  onArchive,\n}: {', '  onArchive,\n  leadingAction,\n}: {')
      .replace('  onArchive?: () => void;\n}) {', '  onArchive?: () => void;\n  leadingAction?: ReactNode;\n}) {')
      .replace('<div className="flex min-w-0 items-center gap-1.5 text-caption text-muted-foreground">', '<div className="flex min-w-0 items-center gap-1.5 text-caption text-muted-foreground">\n          {leadingAction}')
      .replace('function DetailLoadingSkeleton() {', 'function DetailLoadingSkeleton({ leading }: { leading?: ReactNode } = {}) {')
      .replace('<div className="shrink-0 border-b px-6 pb-5 pt-3">\n        <Skeleton className="h-4 w-48" />', '<div className="shrink-0 border-b px-6 pb-5 pt-3">\n        <div className="mb-3 flex items-center gap-2">\n          {leading ?? <Skeleton className="h-4 w-48" />}\n        </div>')
    if (!text.includes('leadingAction?: ReactNode') || !text.includes('leadingAction={leadingAction}') || !text.includes('leading ??')) throw new Error("Agent detail leading slot source changed");
    changes.push("Expose a host return control for agent detail and loading state");
  }
  if (name === "packages/views/modals/run-confirm.tsx") {
    const marker = "    return handoffSupported(readRuntimeCliVersion(runtime.metadata));";
    if (!text.includes(marker)) throw new Error("Run-confirm handoff gate source changed");
    text = text.replace(marker, "    return runtime.metadata?.nativeHandoffSupported === true || handoffSupported(readRuntimeCliVersion(runtime.metadata));");
    changes.push("Accept verified native handoff capability without a synthetic CLI version");
  }
  if (name === "packages/core/platform/storage.ts") {
    text = text.replace(/localStorage\.(getItem|setItem|removeItem)\(k/g, 'localStorage.$1("ccp.workflow." + k');
    changes.push("Namespace durable UI preferences away from the Codex host");
  }
  if (name === "packages/ui/styles/tokens.css") {
    text = text.replace(/:root/g, ":host, .ccp-workflow-surface");
    text = text.replace(/\.dark\s*\{/g, ".dark, .dark .ccp-workflow-surface {");
    changes.push("Apply upstream token defaults inside the Shadow DOM host");
  }
  if (imports.size) text = `import { ${[...imports].join(", ")} } from "@ccp/workflow-host";\n${text}`;
  const webhook = adaptWebhookUi(name, text);
  if (webhook) { text = webhook.text; changes.push(webhook.change); }
  if (changes.length && !name.endsWith(".json")) text = `/* CCP modification: ${changes.join("; ")}. Upstream attribution: vendor/multica/NOTICE. */\n${text}`;
  return { output: changes.length ? Buffer.from(text) : data, changes };
}
const manifest = [];
const previousManifestPath = path.join(destination, "source-files.json");
if (fs.existsSync(previousManifestPath)) {
  const previous = JSON.parse(fs.readFileSync(previousManifestPath, "utf8"));
  for (const entry of previous.files) {
    if (selected.has(entry.source)) continue;
    const obsolete = path.resolve(destination, entry.source);
    if (!obsolete.startsWith(destination + path.sep)) throw new Error("Invalid previous manifest path");
    fs.rmSync(obsolete, { force: true });
  }
}
for (const name of [...selected].sort()) {
  const data = blobs.get(name);
  const target = path.join(destination, name);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  const { output, changes } = adapt(name, data);
  fs.writeFileSync(target, output);
  manifest.push({ source: name, sha256: sha(data), destination: `vendor/multica/${name}`, derivedSha256: sha(output), changes });
}
fs.writeFileSync(path.join(destination, "source-files.json"), JSON.stringify({ revision, files: manifest }, null, 2) + "\n");
console.log(`Vendored ${manifest.length} files from ${revision}. No upstream server, app or runtime copied.`);
