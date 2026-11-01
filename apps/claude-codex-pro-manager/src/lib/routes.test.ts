import { describe, expect, test } from "vitest";

import {
  isRoute,
  normalizeRoute,
  primaryRoute,
  routes,
  routeCatalog,
  routeDocumentTitle,
  routeLabel,
  routeSubtitle,
} from "@/lib/routes";
import type { Route } from "@/types";

/*
 * The navigation's contract, not its looks.
 *
 * The number here is mirrored by the Rust contract test in
 * `src-tauri/tests/windows_subsystem.rs`, which reads the same file and asserts the same
 * count: the sidebar is what a person navigates by, and an entry that appears in one place
 * without the other is a screen nobody can reach or a link that goes nowhere.
 */
const PRIMARY_DESTINATIONS = 10;

describe("导航表", () => {
  test("侧栏的主目的地数量与 Rust 契约一致", () => {
    expect(routes.length).toBe(PRIMARY_DESTINATIONS);
  });

  test("每一项都可供人阅读，且 id 不重复", () => {
    const ids = routes.map((route) => route.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const route of routes) {
      expect(route.label.trim().length).toBeGreaterThan(0);
      expect(route.description.trim().length).toBeGreaterThan(0);
      expect(route.keywords.length).toBeGreaterThan(0);
    }
  });

  /*
   * The CCP-Bot page was removed on purpose. This is the assertion that keeps a refactor from
   * quietly bringing the entry back: the Rust commands behind it still exist, so a restored
   * navigation row would compile and look fine while pointing at a screen we deleted.
   */
  test("被移除的 CCP-Bot 页不会重新出现在导航里", () => {
    // Compared through `string`: the route union no longer contains it, which is the point of the
    // test, so a direct comparison would be a type error rather than a failing assertion.
    const removed = "ccpbot";
    expect(routes.some((route) => String(route.id) === removed)).toBe(false);
    expect(routeCatalog.some((route) => String(route.id) === removed)).toBe(false);
    for (const route of routes) {
      expect(route.label).not.toContain("CCP-Bot");
    }
  });

  test("每个目的地都能被解析成标题、面包屑与文档标题", () => {
    for (const { id } of routes) {
      expect(isRoute(id)).toBe(true);
      expect(primaryRoute(id)).toBe(id);
      expect(routeLabel(id).trim().length).toBeGreaterThan(0);
      expect(typeof routeSubtitle(id)).toBe("string");
      expect(routeDocumentTitle(id)).toContain("管理工具");
    }
  });

  test("旧地址仍然落在对应的新页面上", () => {
    // Legacy ids are normalised on the way in, so a bookmark or a stored route keeps working.
    const landed = (legacy: string) => normalizeRoute(legacy) as Route;
    expect(landed("relay")).toBe("supplier");
    expect(landed("models")).toBe("supplier");
    expect(landed("logs")).toBe("settings");
    expect(landed("pluginHub")).toBe("tools");
    // The two compatibility entries are the ones `primaryRoute` folds back into settings.
    expect(primaryRoute("maintenance")).toBe("settings");
    expect(primaryRoute("about")).toBe("settings");
  });
});