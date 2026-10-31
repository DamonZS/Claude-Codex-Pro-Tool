import { afterEach, describe, expect, it } from "vitest";

import { initialRoute, isRoute, routeLabel, routes } from "@/lib/routes";

declare global {
  // eslint-disable-next-line no-var
  var __CLAUDE_CODEX_PRO_INITIAL_ROUTE: unknown;
}

function withSearch(search: string) {
  window.history.replaceState({}, "", `/${search}`);
}

afterEach(() => {
  delete (window as { __CLAUDE_CODEX_PRO_INITIAL_ROUTE?: unknown }).__CLAUDE_CODEX_PRO_INITIAL_ROUTE;
  withSearch("");
});

describe("CCP-Bot 作为一等路由", () => {
  it("在侧栏里有一个主目的地，带标签、图标与关键词", () => {
    const item = routes.find((route) => route.id === "ccpbot");
    expect(item).toBeTruthy();
    expect(item?.label).toBe("CCP-Bot");
    expect(item?.icon).toBeTruthy();
    expect(item?.keywords).toContain("ccpbot");
    expect(isRoute("ccpbot")).toBe(true);
    expect(routeLabel("ccpbot")).toBe("CCP-Bot");
  });

  it("排在内置目的地里、且是主目的地而不是兼容路由", () => {
    const ids = routes.map((route) => route.id);
    expect(ids).toContain("ccpbot");
    // The compatibility list is for legacy names (about/maintenance): CCP-Bot is a
    // destination people are meant to click, not one that only exists for old links.
    expect(ids.filter((id) => id === "ccpbot")).toHaveLength(1);
  });

  it("注入的初始路由优先，其次是 ?view=，最后才回到 overview", () => {
    withSearch("");
    expect(initialRoute()).toBe("overview");

    withSearch("?view=ccpbot");
    expect(initialRoute()).toBe("ccpbot");

    withSearch("");
    (window as { __CLAUDE_CODEX_PRO_INITIAL_ROUTE?: unknown }).__CLAUDE_CODEX_PRO_INITIAL_ROUTE = "ccpbot";
    expect(initialRoute()).toBe("ccpbot");

    (window as { __CLAUDE_CODEX_PRO_INITIAL_ROUTE?: unknown }).__CLAUDE_CODEX_PRO_INITIAL_ROUTE = "about";
    // Legacy names still normalize rather than falling off the map.
    expect(initialRoute()).toBe("settings");
  });
});