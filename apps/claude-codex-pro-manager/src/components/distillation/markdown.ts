/**
 * Minimal, escape-first markdown renderer for distilled package previews.
 * All input is HTML-escaped before any markup is produced, so model output
 * can never inject tags or attributes. Links are limited to http(s).
 */
function escapeHtml(text: string) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function inline(text: string) {
  return escapeHtml(text)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>")
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer noopener">$1</a>');
}

function tableRow(line: string, cell: "td" | "th") {
  const cells = line.trim().replace(/^\||\|$/g, "").split("|");
  return `<tr>${cells.map((value) => `<${cell}>${inline(value.trim())}</${cell}>`).join("")}</tr>`;
}

export function renderMarkdown(source: string): string {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const out: string[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index]!;
    const fence = /^```(\w*)/.exec(line);
    if (fence) {
      const body: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index]!.startsWith("```")) body.push(lines[index++]!);
      index += 1;
      out.push(`<pre><code>${escapeHtml(body.join("\n"))}</code></pre>`);
      continue;
    }
    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading) {
      const level = heading[1]!.length;
      out.push(`<h${level}>${inline(heading[2]!)}</h${level}>`);
      index += 1;
      continue;
    }
    if (/^\s*\|.*\|\s*$/.test(line) && /^\s*\|?\s*:?-{2,}/.test(lines[index + 1] ?? "")) {
      const rows = [tableRow(line, "th")];
      index += 2;
      while (index < lines.length && /^\s*\|.*\|\s*$/.test(lines[index]!)) rows.push(tableRow(lines[index++]!, "td"));
      out.push(`<table>${rows.join("")}</table>`);
      continue;
    }
    if (/^\s*([-*+]|\d+\.)\s+/.test(line)) {
      const ordered = /^\s*\d+\./.test(line);
      const items: string[] = [];
      while (index < lines.length && /^\s*([-*+]|\d+\.)\s+/.test(lines[index]!)) {
        items.push(`<li>${inline(lines[index]!.replace(/^\s*([-*+]|\d+\.)\s+/, ""))}</li>`);
        index += 1;
      }
      out.push(ordered ? `<ol>${items.join("")}</ol>` : `<ul>${items.join("")}</ul>`);
      continue;
    }
    if (/^>\s?/.test(line)) {
      const quote: string[] = [];
      while (index < lines.length && /^>\s?/.test(lines[index]!)) quote.push(inline(lines[index++]!.replace(/^>\s?/, "")));
      out.push(`<blockquote>${quote.join("<br>")}</blockquote>`);
      continue;
    }
    if (/^(-{3,}|\*{3,})\s*$/.test(line)) {
      out.push("<hr>");
      index += 1;
      continue;
    }
    if (!line.trim()) {
      index += 1;
      continue;
    }
    const paragraph: string[] = [];
    while (index < lines.length && lines[index]!.trim() && !/^(#{1,6}\s|```|>|\s*([-*+]|\d+\.)\s)/.test(lines[index]!)) {
      paragraph.push(inline(lines[index++]!));
    }
    out.push(`<p>${paragraph.join("<br>")}</p>`);
  }
  return out.join("");
}
