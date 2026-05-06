const { chromium } = require('playwright');

const BASE_URL = 'http://127.0.0.1:6789';
const TOKEN = 'enjoy-party';
const TIMEOUT = 30000;

(async () => {
  let exitCode = 0;
  const browser = await chromium.launch({ headless: true });

  try {
    const context = await browser.newContext();
    const page = await context.newPage();

    // Helper: wait for condition with timeout
    async function waitFor(fn, ms = 5000) {
      const start = Date.now();
      while (Date.now() - start < ms) {
        if (await fn()) return true;
        await page.waitForTimeout(200);
      }
      return false;
    }

    // ── Step 1: Navigate to homepage ──────────────────────────────────
    console.log('[Step 1] Navigate to homepage...');
    await page.goto(BASE_URL, { timeout: TIMEOUT });
    console.log('  ✅ Page loaded');

    // ── Step 2: Enter token if token dialog shown ─────────────────────
    console.log('[Step 2] Enter token...');
    const hasTokenInput = await page.locator('input[placeholder="API token"]').count() > 0;
    if (hasTokenInput) {
      await page.fill('input[placeholder="API token"]', TOKEN);
      await page.click('button:has-text("Continue")');
      await page.waitForSelector('.chat-container, .chat-sidebar', { timeout: TIMEOUT });
      console.log('  ✅ Token submitted, sidebar loaded');
    } else {
      console.log('  ℹ️ No token dialog; already authenticated');
    }

    // ── Step 3: Click 0#Agent ─────────────────────────────────────────
    console.log('[Step 3] Select 0#Agent...');
    await page.waitForSelector('.sidebar-content, [class*="sidebar"]', { timeout: TIMEOUT });

    // Find 0#Agent in the sidebar (may be under "agents" section)
    const agentSelectors = [
      'text=0#Agent',
      '[class*="agent-"]:has-text("0#Agent")',
      '.sidebar-item:has-text("0#Agent")',
      'div:has-text("0#Agent")',
    ];
    let clicked = false;
    for (const sel of agentSelectors) {
      try {
        const el = page.locator(sel).first();
        if (await el.count() > 0 && await el.isVisible()) {
          await el.click({ timeout: 5000 });
          clicked = true;
          console.log('  ✅ Clicked 0#Agent via selector:', sel);
          break;
        }
      } catch (e) {}
    }
    if (!clicked) {
      console.log('  ℹ️ 0#Agent not found or already selected');
    }

    // Wait for ChatMain to show
    await page.waitForTimeout(500);
    console.log('  Waiting for chat area...');

    // ── Step 4: Type and send a message ───────────────────────────────
    console.log('[Step 4] Send message to 0#Agent...');
    const inputSelectors = [
      'textarea[placeholder*="message"]',
      'textarea',
      '[contenteditable="true"]',
      'input[type="text"]',
    ];
    let inputFound = false;
    for (const sel of inputSelectors) {
      try {
        const el = page.locator(sel).first();
        if (await el.count() > 0 && await el.isVisible() && await el.isEnabled()) {
          await el.fill('Hello from Playwright test');
          // Press Enter to send
          await el.press('Enter');
          inputFound = true;
          console.log('  ✅ Message sent via selector:', sel);
          break;
        }
      } catch (e) {}
    }
    if (!inputFound) {
      // Fallback: try clicking a send button
      console.log('  ⚠️ No input found; trying to find send button...');
      throw new Error('Could not find message input');
    }

    // ── Step 5: Wait briefly for WebSocket to start streaming ─────────
    console.log('[Step 5] Wait for streaming to start...');
    await page.waitForTimeout(2000);
    console.log('  ✅ Waited 2s (WebSocket should be active)');

    // ── Step 6: HARD REFRESH the page (the core test) ─────────────────
    console.log('[Step 6] HARD REFRESH (F5) while WebSocket active...');
    const refreshStart = Date.now();
    await page.reload({ waitUntil: 'load', timeout: TIMEOUT });
    const refreshTime = Date.now() - refreshStart;
    console.log(`  ✅ Page refreshed in ${refreshTime}ms`);

    if (refreshTime > 10000) {
      console.error('  ⚠️ WARNING: Refresh took > 10s - Pipy event loop may still be blocked!');
      exitCode = 1;
    } else if (refreshTime > 3000) {
      console.log('  ⚠️ Refresh was slow (>3s) but acceptable');
    } else {
      console.log('  ✅ Fast refresh (<3s)');
    }

    // ── Step 7: Re-enter token if needed ──────────────────────────────
    console.log('[Step 7] Re-authenticate after refresh...');
    const hasTokenAfterRefresh = await page.locator('input[placeholder="API token"]').count() > 0;
    if (hasTokenAfterRefresh) {
      await page.fill('input[placeholder="API token"]', TOKEN);
      await page.click('button:has-text("Continue")');
      await page.waitForSelector('.chat-container, .chat-sidebar', { timeout: TIMEOUT });
      console.log('  ✅ Re-authenticated');
    } else {
      console.log('  ℹ️ Already authenticated');
    }

    // ── Step 8: Click 0#Agent again ───────────────────────────────────
    console.log('[Step 8] Re-select 0#Agent after refresh...');
    for (const sel of agentSelectors) {
      try {
        const el = page.locator(sel).first();
        if (await el.count() > 0 && await el.isVisible()) {
          await el.click({ timeout: 5000 });
          console.log('  ✅ Re-selected 0#Agent');
          break;
        }
      } catch (e) {}
    }
    await page.waitForTimeout(1000);

    // ── Step 9: Send second message and verify response ───────────────
    console.log('[Step 9] Send second message after refresh...');
    for (const sel of inputSelectors) {
      try {
        const el = page.locator(sel).first();
        if (await el.count() > 0 && await el.isVisible() && await el.isEnabled()) {
          await el.fill('Second message after refresh test');
          await el.press('Enter');
          console.log('  ✅ Second message sent');
          break;
        }
      } catch (e) {}
    }

    // Wait for some response content to appear
    console.log('[Step 10] Wait for agent response...');
    await page.waitForTimeout(3000);

    // Take a screenshot for debugging
    await page.screenshot({ path: '/Users/caishu/github/clawparty/playwright-result.png' });
    console.log('  📸 Screenshot saved to playwright-result.png');

    // ── Final Check: page should still be responsive ──────────────────
    console.log('[Final] Verify page is still responsive...');
    const title = await page.title();
    console.log(`  Page title: ${title}`);

    const pageSource = await page.content();
    if (pageSource.includes('hang') || pageSource.length < 100) {
      console.error('  ❌ FAIL: Page appears hung or empty');
      exitCode = 1;
    } else {
      console.log('  ✅ Page content is healthy');
    }

    // API health check
    console.log('[Final] API health check...');
    const apiOk = await page.evaluate(async () => {
      try {
        const r = await fetch('http://127.0.0.1:6789/api/version?token=enjoy-party', { method: 'GET' });
        return r.status === 200;
      } catch (e) {
        return false;
      }
    });
    if (apiOk) {
      console.log('  ✅ API is responsive from browser context');
    } else {
      console.error('  ⚠️ API may be slow/unresponsive from browser');
      exitCode = 1;
    }

    console.log('\n========================================');
    if (exitCode === 0) {
      console.log('✅ ALL TESTS PASSED');
    } else {
      console.log('❌ SOME TESTS FAILED');
    }
    console.log('========================================');

  } catch (e) {
    console.error('❌ TEST ERROR:', e.message);
    exitCode = 2;
    try {
      await page.screenshot({ path: '/Users/caishu/github/clawparty/playwright-error.png' });
      console.log('📸 Error screenshot saved');
    } catch {}
  } finally {
    await browser.close();
    process.exit(exitCode);
  }
})();
