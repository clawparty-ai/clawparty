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

    // Flexible wait: look for any content rather than a specific class
    async function waitForPageReady() {
      await page.waitForFunction(() => {
        const body = document.body.innerText || '';
        return body.length > 50 || document.querySelectorAll('div').length > 5;
      }, { timeout: TIMEOUT });
    }

    // ── Step 1: Load page ─────────────────────────────────────────────
    console.log('[Stress Test] Navigate to homepage...');
    await page.goto(BASE_URL, { timeout: TIMEOUT });
    await waitForPageReady();
    console.log('  ✅ Page loaded');

    // ── Step 2: Run 5 cycles of send + immediate refresh ──────────────
    console.log(`[Stress Test] Running 5 cycles of send → refresh...`);
    for (let cycle = 1; cycle <= 5; cycle++) {
      console.log(`\n--- Cycle ${cycle} ---`);

      // Click 0#Agent
      try {
        const el = page.locator('text=0#Agent').first();
        if (await el.count() > 0 && await el.isVisible()) {
          await el.click();
        }
      } catch (e) {}
      await page.waitForTimeout(300);

      // Send message
      try {
        const ta = page.locator('textarea').first();
        if (await ta.count() > 0 && await ta.isVisible() && await ta.isEnabled()) {
          await ta.fill('Cycle ' + cycle + ' stress test');
          await ta.press('Enter');
          console.log(`  ✅ Message sent (cycle ${cycle})`);
        }
      } catch (e) {
        console.log(`  ⚠️ Could not send message (cycle ${cycle}): ${e.message}`);
      }

      // Wait 1 second (WebSocket should be streaming)
      await page.waitForTimeout(1000);

      // HARD REFRESH
      const start = Date.now();
      try {
        await page.reload({ waitUntil: 'load', timeout: TIMEOUT });
        const t = Date.now() - start;
        console.log(`  ✅ Refresh in ${t}ms (cycle ${cycle})`);
        if (t > 5000) {
          console.error(`  ❌ SLOW: Refresh took ${t}ms - event loop blocked!`);
          exitCode = 1;
        }
      } catch (e) {
        console.error(`  ❌ Refresh FAILED (cycle ${cycle}): ${e.message}`);
        exitCode = 1;
        break;
      }

      // Wait for page to be ready again
      await waitForPageReady();
    }

    // ── Step 3: API soak test ─────────────────────────────────────────
    console.log(`\n[Stress Test] API soak: 20 rapid /api/agents requests...`);
    let slowCount = 0;
    for (let i = 0; i < 20; i++) {
      const start = Date.now();
      const ok = await page.evaluate(async () => {
        try {
          const r = await fetch('http://127.0.0.1:6789/api/agents?token=enjoy-party');
          return r.status === 200;
        } catch (e) { return false; }
      });
      const t = Date.now() - start;
      if (!ok) {
        console.error(`  ❌ Request ${i + 1} failed`);
        exitCode = 1;
      } else if (t > 1000) {
        slowCount++;
        console.log(`  ⚠️ Request ${i + 1}: ${t}ms (slow)`);
      }
      await page.waitForTimeout(100);
    }
    console.log(`  ✅ 20 requests done, ${slowCount} slow (>1s)`);
    if (slowCount > 5) {
      console.error('  ❌ Too many slow requests');
      exitCode = 1;
    }

    // ── Final screenshot ──────────────────────────────────────────────
    await page.screenshot({ path: '/Users/caishu/github/clawparty/playwright-stress.png' });
    console.log('\n📸 Screenshot saved to playwright-stress.png');

    console.log('\n========================================');
    if (exitCode === 0) {
      console.log('✅ ALL STRESS TESTS PASSED');
    } else {
      console.log('❌ SOME STRESS TESTS FAILED');
    }
    console.log('========================================');

  } catch (e) {
    console.error('❌ TEST ERROR:', e.message);
    exitCode = 2;
    try {
      await page.screenshot({ path: '/Users/caishu/github/clawparty/playwright-stress-error.png' });
    } catch {}
  } finally {
    await browser.close();
    process.exit(exitCode);
  }
})();
