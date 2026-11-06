export type SupplierSharePayload = {
  name: string;
  baseUrl: string;
  upstreamBaseUrl: string;
  model: string;
  targetApp: string;
};

export const SUPPLIER_SHARE_PREFIX = "ccp://supplier?d=";

const toBase64Url = (bytes: Uint8Array) => {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};

const fromBase64Url = (value: string) => {
  const binary = atob(value.replace(/-/g, "+").replace(/_/g, "/"));
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
  return bytes;
};

/** 把供应商的关键字段编成一条可粘贴的链接。刻意不含 API Key。 */
export function encodeSupplierShareLink(profile: {
  name?: string;
  baseUrl?: string;
  upstreamBaseUrl?: string;
  model?: string;
  testModel?: string;
  targetApp?: string;
}) {
  const payload: SupplierSharePayload = {
    name: (profile.name ?? "").trim(),
    baseUrl: (profile.upstreamBaseUrl || profile.baseUrl || "").trim(),
    upstreamBaseUrl: (profile.upstreamBaseUrl || "").trim(),
    model: (profile.testModel || profile.model || "").trim(),
    targetApp: (profile.targetApp ?? "").trim(),
  };
  return SUPPLIER_SHARE_PREFIX + toBase64Url(new TextEncoder().encode(JSON.stringify(payload)));
}

/** 解析链接；不是本格式或字段为空时返回 null。 */
export function decodeSupplierShareLink(link: string): SupplierSharePayload | null {
  const trimmed = link.trim();
  if (!trimmed.startsWith(SUPPLIER_SHARE_PREFIX)) return null;
  const encoded = trimmed.slice(SUPPLIER_SHARE_PREFIX.length).split(/\s/)[0] ?? "";
  if (!encoded) return null;
  try {
    const parsed: unknown = JSON.parse(new TextDecoder().decode(fromBase64Url(encoded)));
    if (!parsed || typeof parsed !== "object") return null;
    const source = parsed as Record<string, unknown>;
    const read = (key: string) => (typeof source[key] === "string" ? (source[key] as string).trim() : "");
    const baseUrl = read("baseUrl");
    if (!baseUrl) return null;
    return {
      name: read("name"),
      baseUrl,
      upstreamBaseUrl: read("upstreamBaseUrl"),
      model: read("model"),
      targetApp: read("targetApp"),
    };
  } catch {
    return null;
  }
}