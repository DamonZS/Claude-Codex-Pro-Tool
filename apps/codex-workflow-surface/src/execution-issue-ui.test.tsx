import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useCreateIssueSurfaceSelection } from "@multica/views/issues/surface/selection-context";
import { getMoveAnchors } from "@multica/views/issues/utils/drag-utils";

describe("original IssueSurface readonly selection", () => {
  it("keeps ordinary drag neighbors on real issue IDs across virtual cards", () => {
    expect(getMoveAnchors(["before", "codex-native:child", "ordinary", "ccp-execution:binding", "after"], "ordinary")).toEqual({ before_id: "before", after_id: "after" });
    expect(getMoveAnchors(["codex-native:child", "ordinary", "ccp-execution:binding"], "ordinary")).toEqual({ before_id: null, after_id: null });
  });
  it("excludes execution projections from row toggles, select-all and ranges", () => {
    const { result } = renderHook(() => useCreateIssueSurfaceSelection("fixture"));
    act(() => result.current.toggle("codex-native:child"));
    expect([...result.current.selectedIds]).toEqual([]);
    act(() => result.current.select(["ordinary", "ccp-execution:binding", "codex-native:child"]));
    expect([...result.current.selectedIds]).toEqual(["ordinary"]);
    act(() => result.current.toggle("ordinary"));
    expect([...result.current.selectedIds]).toEqual([]);
  });
});
