const fs = require('node:fs');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');
const source = fs.readFileSync('assets/inject/renderer-inject.js', 'utf8');
const names = ['escapeHtml', 'confirmDelete', 'releaseDeleteFocus', 'openDeleteConfirmForRow', 'installDeleteButtonEventDelegation', 'stopActionButtonEvent', 'installActionButtonEvents'];
const functions = names.map(name => {
  const start = source.indexOf(`  function ${name}(`);
  assert.ok(start >= 0, name);
  const end = source.indexOf('\n  }', start) + 4;
  return source.slice(start, end);
}).join('\n');
(async () => {
  const browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' });
  try {
    const page = await browser.newPage();
    await page.setContent('<div data-app-action-sidebar-thread-id="fixture"><button class="codex-delete-button">Delete</button></div>');
    await page.addScriptTag({ content: `
      const buttonClass = 'codex-delete-button';
      const sessionRefFromRow = () => ({session_id: 'fixture', title: 'Fixture'});
      const showActionButtonTooltip = () => {};
      const hideActionButtonTooltip = () => {};
      window.calls = []; window.dialogCount = 0;
      const postJson = async (path, ref) => {
        calls.push({path, ref});
        if (window.resultStatus === 'pending') return new Promise(() => {});
        return {status: window.resultStatus || 'local_deleted', message: window.resultStatus === 'failed' ? 'fixture failure' : ''};
      };
      const showToast = (message) => { window.toast = message; };
      const sendClaudeCodexProDiagnostic = () => {};
      const removeDeletedRow = row => row.remove();
      ${functions}
      const createElement = document.createElement.bind(document);
      document.createElement = (...args) => { if(args[0] === 'div') window.dialogCount++; return createElement(...args); };
      const row = document.querySelector('[data-app-action-sidebar-thread-id]');
      const button = row.querySelector('button');
      installActionButtonEvents(row, button, event => openDeleteConfirmForRow(row, button, sessionRefFromRow(row), event));
      installDeleteButtonEventDelegation();
    ` });
    // Dispatch both events to retain the same target across the overlay insertion.
    await page.locator('.codex-delete-button').dispatchEvent('pointerup');
    await page.locator('.codex-delete-button').dispatchEvent('click');
    assert.equal(await page.evaluate(() => dialogCount), 1, 'one gesture opens one dialog');
    await page.locator('[data-codex-delete-cancel]').click();
    assert.equal(await page.evaluate(() => calls.length), 0);
    await page.evaluate(() => { window.resultStatus = 'failed'; });
    await page.locator('.codex-delete-button').click();
    await page.locator('[data-codex-delete-confirm]').click();
    assert.equal(await page.evaluate(() => calls.length), 1);
    assert.equal(await page.evaluate(() => toast), 'fixture failure');
    assert.equal(await page.locator('[data-app-action-sidebar-thread-id]').count(), 1);
    await page.clock.install();
    await page.evaluate(() => { window.resultStatus = 'pending'; });
    await page.locator('.codex-delete-button').click();
    await page.locator('[data-codex-delete-confirm]').click();
    assert.equal(await page.locator('.codex-delete-button').isDisabled(), true);
    await page.clock.fastForward(15001);
    assert.equal(await page.locator('.codex-delete-button').isDisabled(), false);
    assert.equal(await page.locator('[data-app-action-sidebar-thread-id]').count(), 1);
    assert.equal(await page.evaluate(() => calls.length), 2, 'timeout must not retry deletion');
    assert.match(await page.evaluate(() => toast), /删除结果尚未确认/);
    await page.evaluate(() => { window.resultStatus = 'local_deleted'; });
    await page.locator('.codex-delete-button').click();
    await page.locator('[data-codex-delete-confirm]').click();
    assert.equal(await page.evaluate(() => calls.length), 3);
    assert.equal(await page.locator('[data-app-action-sidebar-thread-id]').count(), 0);
    console.log('PASS: gesture deduplication, cancel, failure, timeout without retry, single delete, row removal');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
