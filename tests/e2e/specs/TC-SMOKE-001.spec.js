import { test, expect } from '@playwright/test';

/**
 * TC-SMOKE-001: GUI Smoke Test
 *
 * Purpose: Verify that the GUI loads correctly and basic elements are visible.
 * This is a quick sanity check before running full E2E tests.
 */
test.describe('TC-SMOKE-001: GUI Smoke Test', () => {
  test('should load GUI and display main elements', async ({ page }) => {
    console.log('[TC-SMOKE-001] Step 1: Opening GUI...');
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    console.log('[TC-SMOKE-001] Step 2: Waiting for Vue app to mount...');
    // Wait for Vue app to mount - #app should have children
    await page.waitForFunction(
      () => {
        const app = document.querySelector('#app');
        return app && app.children.length > 0;
      },
      { timeout: 15000 }
    );

    console.log('[TC-SMOKE-001] Step 3: Checking for token dialog or main app...');
    // Check if token dialog is shown (no saved token) or main app is shown
    const tokenDialog = page.locator('.token-dialog');
    const orgRail = page.locator('.org-rail');

    const tokenDialogVisible = await tokenDialog.isVisible().catch(() => false);
    const orgRailVisible = await orgRail.isVisible().catch(() => false);

    // Either token dialog or main app should be visible
    expect(tokenDialogVisible || orgRailVisible).toBe(true);

    if (tokenDialogVisible) {
      console.log('[TC-SMOKE-001] Token dialog is visible - GUI loaded correctly');

      // Verify token dialog elements
      const tokenInput = page.locator('.token-dialog input[type="password"]');
      const tokenSubmitBtn = page.locator('.token-dialog button');

      await expect(tokenInput).toBeVisible();
      await expect(tokenSubmitBtn).toBeVisible();

      console.log('[TC-SMOKE-001] Submitting test token...');
      await tokenInput.fill('test-token');
      await tokenSubmitBtn.click();

      // Wait for either token dialog to hide or org-rail to appear
      await Promise.race([
        tokenDialog.waitFor({ state: 'hidden', timeout: 15000 }),
        orgRail.waitFor({ state: 'visible', timeout: 15000 })
      ]);
    }

    console.log('[TC-SMOKE-001] Step 4: Verifying main app elements...');
    // Now main app should be visible
    await orgRail.waitFor({ state: 'visible', timeout: 15000 });

    // Verify key UI elements exist
    const joinPartyBtn = page.locator('button.new-group-rail-btn[title="加入组织"]');
    const zagentsIcon = page.locator('.org-icon[title="zAgents"]');

    await expect(orgRail).toBeVisible();
    await expect(joinPartyBtn).toBeVisible();
    await expect(zagentsIcon).toBeVisible();

    console.log('[TC-SMOKE-001] Test completed successfully - GUI loaded correctly');
  });
});
