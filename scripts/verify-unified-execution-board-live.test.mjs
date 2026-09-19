import assert from "node:assert/strict";
import test from "node:test";
import { verifySourceCards } from "./verify-unified-execution-board-live.mjs";

const sources = {
  issues: [{ id: "linked" }],
  executions: [
    { id: "old", issue: "linked", thread: "old-thread", state: "completed", created: 9, attempt: 10 },
    { id: "new", issue: "linked", thread: "new-thread", state: "binding_pending", created: 10, attempt: 1 },
    { id: "standalone", thread: "standalone-thread", state: "running", created: 10, attempt: 1 },
  ],
  native: [
    { id: "old-thread", state: "completed" }, { id: "new-thread", state: "inProgress" },
    { id: "standalone-thread", state: "inProgress" }, { id: "child", state: "completed" },
  ],
};
const cards = [
  { id: "linked", state: "binding_pending", source: "multica-execution", column: "todo" },
  { id: "ccp-execution:standalone", state: "running", source: "multica-execution", column: "in_progress" },
  { id: "codex-native:child", state: "completed", source: "codex-native", column: "done" },
];
test("resumed native bindings preserve the original card identity and queued state", () => {
  const resumed = { issues: [], native: [{ id: "child", state: "completed" }], executions: [
    { id: "binding", thread: "child", state: "binding_pending", nativeResume: true, created: 10 },
  ] };
  assert.deepEqual(verifySourceCards([{ id: "codex-native:child", state: "binding_pending", source: "codex-native", column: "todo" }], resumed), { projectedTotal: 1, renderedChecked: 1 });
});
test("matches independent readonly source snapshots including latest binding and thread dedup", () => {
  assert.deepEqual(verifySourceCards(cards, sources), { projectedTotal: 3, renderedChecked: 3 });
});
test("rejects internally consistent UI carrying an obsolete source state", () => {
  assert.throws(() => verifySourceCards([{ ...cards[0], state: "completed", column: "done" }], sources), /source_state_mismatch/);
});
test("rejects duplicate native projections of a bound execution", () => {
  assert.throws(() => verifySourceCards([...cards, { id: "codex-native:old-thread", state: "completed", source: "codex-native", column: "done" }], sources), /unexpected_projected_card/);
});
test("rejects correct state in the wrong column", () => {
  assert.throws(() => verifySourceCards([{ ...cards[2], column: "in_progress" }], sources), /source_column_mismatch/);
});
test("requires selected source subjects even when other columns are virtualized", () => {
  assert.throws(() => verifySourceCards(cards.slice(0, 2), sources, ["codex-native:child"]), /required_source_card_missing/);
});
test("keeps directly linked Issue priority over an unlinked newer run", () => {
  const extra = { id: "other", thread: "new-thread", state: "completed", created: 20 };
  assert.deepEqual(verifySourceCards(cards, { ...sources, executions: [...sources.executions, extra] }), { projectedTotal: 3, renderedChecked: 3 });
});
test("checks persisted Issue thread associations without duplicate native cards", () => {
  const associated = { native: [{ id: "child", state: "inProgress" }], executions: [], issues: [{ id: "issue", thread: "child" }] };
  assert.deepEqual(verifySourceCards([{ id: "issue", state: "inProgress", source: "codex-native", column: "in_progress" }], associated), { projectedTotal: 1, renderedChecked: 1 });
});
