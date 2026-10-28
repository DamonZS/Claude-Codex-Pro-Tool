import type { CardQueryInput, CardSaveInput } from "@/types";

/** Largest page the backend accepts (`MAX_PAGE_SIZE`); the list shows one page. */
export const CARD_PAGE_LIMIT = 200;

/** Default filter query: empty strings mean "no filter" (never `"all"`). */
export const DEFAULT_CARD_QUERY: CardQueryInput = {
  kind: "",
  tag: "",
  keyword: "",
  offset: 0,
  limit: CARD_PAGE_LIMIT,
};

/** Mirrors `CARD_KINDS` in the Rust card library. */
export type CardKind = "experience" | "pitfall" | "guidance" | "architecture";

/** `all` maps to an empty `kind` filter — never to the literal string. */
export type CardKindFilter = "all" | CardKind;

export const CARD_KIND_OPTIONS: ReadonlyArray<{ value: CardKind; label: string; hint: string }> = [
  { value: "experience", label: "经验", hint: "可复用的做法与结论" },
  { value: "pitfall", label: "踩坑", hint: "错误模式与规避方式" },
  { value: "guidance", label: "指导", hint: "规范、约定与操作步骤" },
  { value: "architecture", label: "架构", hint: "结构决策与模块关系" },
];

const CARD_KIND_LABELS: Record<string, string> = {
  experience: "经验",
  pitfall: "踩坑",
  guidance: "指导",
  architecture: "架构",
};

/**
 * The backend normalises unknown kinds to `experience`, so anything unexpected
 * is shown as-is to keep a mismatch visible instead of hiding it.
 */
export function cardKindLabel(kind: string) {
  const normalized = (kind || "").trim().toLowerCase();
  return CARD_KIND_LABELS[normalized] ?? (normalized || "经验");
}

/** Cards only count as a kind when it is one of the four known kinds. */
export function kindCount(kinds: Record<string, number>, kind: string) {
  return kinds[kind] ?? 0;
}

/** Comma separated input → trimmed, de-duplicated tags capped at the backend limit. */
export function parseCardTags(raw: string): string[] {
  const seen = new Set<string>();
  const tags: string[] = [];
  for (const piece of raw.split(/[,，、]/)) {
    const tag = piece.trim().slice(0, 32);
    if (!tag || seen.has(tag.toLowerCase())) continue;
    seen.add(tag.toLowerCase());
    tags.push(tag);
  }
  return tags;
}

export function formatCardTags(tags: readonly string[]) {
  return tags.join("，");
}

/** Draft shared by the optional edit dialog and the inline quick-create form. */
export type CardDraft = {
  kind: CardKind;
  title: string;
  body: string;
  tags: string;
  source: string;
};

export function emptyCardDraft(): CardDraft {
  return { kind: "experience", title: "", body: "", tags: "", source: "" };
}

export function cardDraftFromCard(card: {
  kind: string;
  title: string;
  body: string;
  tags: string[];
  source: string;
}): CardDraft {
  const kind = CARD_KIND_OPTIONS.some((option) => option.value === card.kind)
    ? (card.kind as CardKind)
    : "experience";
  return {
    kind,
    title: card.title,
    body: card.body,
    tags: formatCardTags(card.tags ?? []),
    source: card.source,
  };
}

/**
 * Draft → `CardSaveRequest`. `id` must be `""` to create: the Rust field is a
 * plain `String` (never `null`).
 */
export function cardDraftToSaveInput(draft: CardDraft, id = ""): CardSaveInput {
  return {
    id: id ?? "",
    kind: draft.kind,
    title: draft.title.trim(),
    body: draft.body.trim(),
    tags: parseCardTags(draft.tags),
    source: draft.source.trim() || "manual",
  };
}