import { test, expect } from '@playwright/test';
import { TestEnv } from '../fixtures/test-env.js';
import { TEST_DATA } from '../fixtures/test-data.js';
import { JoinPartyPage } from '../pages/join-party.page.js';
import { ScreenshotHelper } from '../utils/screenshot.js';

test.describe('TC-E-001: Invalid Invite Code', () => {
  let testEnv;
  let screenshot;

  test.beforeAll(async () => {
    testEnv = new TestEnv();
    await testEnv.startZtmAgent();
  });

  test.afterAll(async () => {
    await testEnv.stopZtmAgent();
  });

  test('should show error when using non-existent invite code', async ({ page }) => {
    screenshot = new ScreenshotHelper('TC-E-001');

    // Step 1: Navigate to GUI
    console.log('[TC-E-001] Step 1: Opening GUI...');
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Step 2: Wait for Vue app to load - either token dialog or main app
    console.log('[TC-E-001] Step 2: Waiting for Vue app to mount...');
    await page.waitForFunction(
      () => {
        const app = document.querySelector('#app');
        return app && app.children.length > 0;
      },
      { timeout: 15000 }
    );

    // Handle token dialog if it appears (shown when no API token is saved)
    const tokenDialog = page.locator('.token-dialog');
    const tokenDialogVisible = await tokenDialog.isVisible().catch(() => false);
    if (tokenDialogVisible) {
      console.log('[TC-E-001] Token dialog detected, submitting token...');
      const tokenInput = page.locator('.token-dialog input[type="password"]');
      const tokenSubmitBtn = page.locator('.token-dialog button');

      await tokenInput.fill('test-token');
      await tokenSubmitBtn.click();
      await tokenDialog.waitFor({ state: 'hidden', timeout: 15000 });
      await page.waitForTimeout(1000);
    }

    // Wait for main app sidebar (org-rail) to be visible
    await page.waitForSelector('.org-rail', { timeout: 15000 });

    // Step 3: Open Join Party modal
    console.log('[TC-E-001] Step 3: Opening Join Party modal...');
    const joinPartyPage = new JoinPartyPage(page);
    await joinPartyPage.openModal();
    await screenshot.capture(page, 'join-party-modal-opened');

    // Step 4: Fill form with invalid invite code
    console.log('[TC-E-001] Step 4: Filling form with invalid invite code...');
    await joinPartyPage.fillForm({
      regUrl: TEST_DATA.regServerUrl,
      username: TEST_DATA.testUsername,
      inviteCode: TEST_DATA.invalidInviteCode
    });
    await screenshot.capture(page, 'form-filled-invalid-code');

    // Step 5: Submit and wait for error
    console.log('[TC-E-001] Step 5: Submitting form...');
    await joinPartyPage.submit();

    // Wait for error message
    try {
      await joinPartyPage.waitForError(20000);
      await screenshot.capture(page, 'error-displayed');

      const errorText = await joinPartyPage.getErrorText();
      console.log('[TC-E-001] Error message:', errorText);

      // Verify error message contains relevant text
      expect(errorText).toBeTruthy();
      expect(errorText.toLowerCase()).toMatch(/invalid|邀请码|invite|code|不存在|无效/);

      // Verify modal is still visible (not closed)
      const isModalVisible = await joinPartyPage.isModalVisible();
      expect(isModalVisible).toBe(true);

      console.log('[TC-E-001] Test completed successfully - error displayed as expected');
    } catch (error) {
      await screenshot.capture(page, 'unexpected-state');
      throw new Error(`Expected error message not displayed: ${error.message}`);
    }
  });
});
