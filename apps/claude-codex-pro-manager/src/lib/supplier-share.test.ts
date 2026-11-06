import { describe, expect, it } from "vitest";

import { SUPPLIER_SHARE_PREFIX, decodeSupplierShareLink, encodeSupplierShareLink } from "./supplier-share";

describe("supplier share link", () => {
  it("round-trips a profile with a Chinese name", () => {
    const link = encodeSupplierShareLink({
      name: "中转一号",
      baseUrl: "https://api.example.test/v1",
      upstreamBaseUrl: "https://api.example.test/v1",
      testModel: "gpt-6.1-sol",
      targetApp: "codex",
    });
    expect(link.startsWith(SUPPLIER_SHARE_PREFIX)).toBe(true);
    expect(decodeSupplierShareLink(link)).toEqual({
      name: "中转一号",
      baseUrl: "https://api.example.test/v1",
      upstreamBaseUrl: "https://api.example.test/v1",
      model: "gpt-6.1-sol",
      targetApp: "codex",
    });
  });

  it("never carries an API key", () => {
    const link = encodeSupplierShareLink({
      name: "中转一号",
      baseUrl: "https://api.example.test/v1",
    });
    const decoded = decodeSupplierShareLink(link);
    expect(decoded).not.toBeNull();
    expect(Object.keys(decoded!).sort()).toEqual(["baseUrl", "model", "name", "targetApp", "upstreamBaseUrl"]);
    expect(JSON.stringify(decoded)).not.toContain("sk-");
  });

  it("rejects links from other schemes and broken payloads", () => {
    expect(decodeSupplierShareLink("ccswitch://provider?id=abc")).toBeNull();
    expect(decodeSupplierShareLink(`${SUPPLIER_SHARE_PREFIX}%%%`)).toBeNull();
    expect(decodeSupplierShareLink(`${SUPPLIER_SHARE_PREFIX}${btoa('{"name":"x"}')}`)).toBeNull();
  });
});