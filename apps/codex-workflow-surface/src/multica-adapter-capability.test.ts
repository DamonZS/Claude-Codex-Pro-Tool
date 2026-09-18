// @vitest-environment node
import { describe, expect, it } from "vitest";
import { quickCreateVersionBlocked } from "../vendor/multica/packages/core/runtimes/cli-version";

describe("original quick-create runtime capability gate", () => {
  it.each([false, true])("accepts explicit native capability with fields=%s and no CLI", (fields) => {
    expect(quickCreateVersionBlocked({ nativeTaskHostSupported: true }, fields)).toBe(false);
  });
  it.each([undefined, {}, { nativeTaskHostSupported: false }, { nativeTaskHostSupported: "true" }, { cli_version: "0.2.20" }])("retains the version gate without native support: %j", (metadata) => {
    expect(quickCreateVersionBlocked(metadata, false)).toBe(true);
    expect(quickCreateVersionBlocked(metadata, true)).toBe(true);
  });
  it("retains the original daemon version thresholds for base and explicit fields", () => {
    expect(quickCreateVersionBlocked({ cli_version: "0.2.21" }, false)).toBe(false);
    expect(quickCreateVersionBlocked({ cli_version: "0.2.21" }, true)).toBe(true);
    expect(quickCreateVersionBlocked({ cli_version: "0.4.3" }, true)).toBe(false);
  });
});
