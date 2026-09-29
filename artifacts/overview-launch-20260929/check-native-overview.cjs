const port = Number(process.argv[2] || 9223);

async function main() {
  const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
  const target = targets.find((item) => item.type === "page" && item.webSocketDebuggerUrl);
  if (!target) throw new Error("No native WebView page target");
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", reject, { once: true });
  });
  let id = 0;
  const pending = new Map();
  ws.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    const task = pending.get(message.id);
    if (task) {
      pending.delete(message.id);
      message.error ? task.reject(new Error(message.error.message)) : task.resolve(message.result);
    }
  });
  function cdp(method, params = {}) {
    return new Promise((resolve, reject) => {
      const next = ++id;
      pending.set(next, { resolve, reject });
      ws.send(JSON.stringify({ id: next, method, params }));
    });
  }
  async function evaluate(expression) {
    const result = await cdp("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
    return result.result.value;
  }
  const state = await evaluate(`(async () => {
    const overviewButton = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === '概览');
    if (overviewButton && !document.querySelector('.overview-main')) overviewButton.click();
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const area = document.querySelector('.overview-main');
    const calendar = document.querySelector('.aitracker-calendar-panel');
    if (!area || !calendar) return { ready: false, title: document.title, overviewButton: !!overviewButton };
    const before = area.scrollTop;
    area.scrollTop = area.scrollHeight;
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const bounds = calendar.getBoundingClientRect();
    const viewport = area.getBoundingClientRect();
    const headers = [...document.querySelectorAll('.aitracker-project-table th')];
    const cells = [...document.querySelectorAll('.aitracker-project-table tbody tr:first-child td')];
    const topButtons = [...document.querySelectorAll('.aitracker-project-segments button')];
    const activeCell = document.querySelector('.aitracker-calendar-grid span[data-level]:not([data-level="0"])');
    const activeBounds = activeCell?.getBoundingClientRect();
    return { ready: true, clientHeight: area.clientHeight, scrollHeight: area.scrollHeight,
      before, after: area.scrollTop, calendarBottom: bounds.bottom, viewportBottom: viewport.bottom,
      calendarVisible: bounds.bottom <= viewport.bottom + 1,
      modelRows: document.querySelectorAll('.aitracker-model-row').length,
      projectRows: document.querySelectorAll('.aitracker-project-table tbody tr').length,
      trendBars: document.querySelectorAll('.aitracker-trend-replacement rect').length,
      columnOffsets: headers.map((header, index) => Math.round(header.getBoundingClientRect().left - (cells[index]?.getBoundingClientRect().left ?? 0))),
      topWidths: topButtons.map((button) => Math.round(button.getBoundingClientRect().width)),
      topGap: topButtons.slice(1).map((button, index) => Math.round(button.getBoundingClientRect().left - topButtons[index].getBoundingClientRect().right)),
      activeCalendarCell: activeBounds ? { x: activeBounds.x + activeBounds.width / 2, y: activeBounds.y + activeBounds.height / 2 } : null };
  })()`);
  if (state.activeCalendarCell) {
    await cdp("Input.dispatchMouseEvent", { type: "mouseMoved", ...state.activeCalendarCell });
    await new Promise((resolve) => setTimeout(resolve, 100));
    state.calendarTooltip = await evaluate("document.querySelector('.aitracker-calendar-tooltip')?.textContent || null");
  }
  const trendPoint = await evaluate(`(() => {
    const hit = document.querySelector('.aitracker-trend-replacement .trend-hit');
    if (!hit) return null;
    hit.scrollIntoView({ block: 'center' });
    const rect = hit.getBoundingClientRect();
    return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
  })()`);
  if (trendPoint) {
    await cdp("Input.dispatchMouseEvent", { type: "mouseMoved", ...trendPoint });
    await new Promise((resolve) => setTimeout(resolve, 100));
    state.trendTooltip = await evaluate(`(() => {
      const tooltip = document.querySelector('.overview-trend-tooltip-html');
      return tooltip ? { text: tooltip.textContent, fontSize: getComputedStyle(tooltip).fontSize, transform: getComputedStyle(tooltip).transform } : null;
    })()`);
  }
  console.log(JSON.stringify(state, null, 2));
  ws.close();
  if (!state.ready || state.scrollHeight <= state.clientHeight || !state.calendarVisible || state.columnOffsets.some((offset) => Math.abs(offset) > 1) || state.topWidths.some((width) => width < 40) || state.topGap.some((gap) => gap < 0)) process.exitCode = 1;
}

main().catch((error) => { console.error(error); process.exitCode = 1; });
