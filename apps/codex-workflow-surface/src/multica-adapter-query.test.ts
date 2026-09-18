// @vitest-environment node
import { describe, expect, it } from "vitest";
import { tableQuery, type QueryContext } from "./multica-adapter-query";
import { type JsonRecord } from "./multica-adapter-dto";
import { IssueTableGroupsResponseSchema } from "../vendor/multica/packages/core/api/schemas";

const query = { scope: { kind: "workspace" }, filters: {}, search: "", sort: { field: "position", direction: "asc" } };
const group = { kind: "property", property_id: "size", include_empty: true };
const context: QueryContext = { userId: "user-1", agents: [], squads: [], tasks: [], properties: [{ id: "size", revision: 1, type: "select", config: { options: [{ id: "small", name: "Small" }, { id: "large", name: "Large" }] } }] };
const issues: JsonRecord[] = [
  { id: "issue-1", revision: 1, properties: { size: "large" } },
  { id: "issue-2", revision: 1, properties: { size: "removed" } },
  { id: "issue-3", revision: 1, properties: {} },
];

describe("upstream property group semantics", () => {
  it("preserves option order, empty choices, unavailable values and upstream cursor keys", async () => {
    const result = await tableQuery("groups", { query, group }, issues, context);
    expect(IssueTableGroupsResponseSchema.safeParse(result).success).toBe(true);
    expect(result.groups).toMatchObject([
      { key: "property:size:value:c21hbGw", count: 0, value: { value: "small", value_state: "value" } },
      { key: "property:size:value:bGFyZ2U", count: 1, value: { value: "large", value_state: "value" } },
      { key: "property:size:unavailable:cmVtb3ZlZA", count: 1, value: { value: "removed", value_state: "unavailable" } },
      { key: "property:size:unset:", count: 1, value: { value: null, value_state: "unset" } },
    ]);
    const resultRows = await tableQuery("rows", { query, group, group_key: "property:size:unavailable:cmVtb3ZlZA" }, issues, context);
    expect(resultRows.rows).toMatchObject([{ issue: { id: "issue-2" } }]);
  });
  it("invalidates cursor after definition changes even when issue revisions are unchanged", async () => {
    const first = await tableQuery("groups", { query, group, page: { limit: 1 } }, issues, context);
    const changed = { ...context, properties: [{ ...context.properties![0], revision: 2 }] };
    await expect(tableQuery("groups", { query, group, page: { limit: 1, cursor: first.next_cursor } }, issues, changed)).rejects.toMatchObject({ status: 409, code: "query_cursor_conflict" });
  });
  it("retains upstream rejection of multi-valued groups and orders checkbox false/true/unset", async () => {
    await expect(tableQuery("groups", { query, group }, issues, { ...context, properties: [{ id: "size", type: "multi_select" }] })).rejects.toMatchObject({ status: 400, code: "property_type_unsupported" });
    const result = await tableQuery("groups", { query, group }, [], { ...context, properties: [{ id: "size", type: "checkbox" }] });
    expect(result.groups).toMatchObject([{ value: { value: false }, count: 0 }, { value: { value: true }, count: 0 }, { value: { value_state: "unset" }, count: 0 }]);
  });
});
