import { test, expect } from '@playwright/test';
import { TestEnv } from '../fixtures/test-env.js';
import { TEST_DATA } from '../fixtures/test-data.js';
import { JoinPartyPage } from '../pages/join-party.page.js';
import { AgentPage } from '../pages/agent.page.js';
import { ChatPage } from '../pages/chat.page.js';
import { ScreenshotHelper } from '../utils/screenshot.js';

test.describe('TC-P-001: Complete Day1 User Journey', () => {
  let testEnv;
  let screenshot;

  test.beforeAll(async () => {
    testEnv = new TestEnv();
    await testEnv.startZtmAgent();
  });

  test.afterAll(async () => {
    await testEnv.stopZtmAgent();
  });

  test('should complete full Day1 flow: join party, create agent, send message', async ({ page }) => {
    screenshot = new ScreenshotHelper('TC-P-001');

    // Step 1: Navigate to GUI
    console.log('[TC-P-001] Step 1: Opening GUI...');
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Step 2: Wait for Vue app to load - either token dialog or main app
    console.log('[TC-P-001] Step 2: Waiting for Vue app to mount...');
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
      console.log('[TC-P-001] Token dialog detected, submitting token...');
      const tokenInput = page.locator('.token-dialog input[type="password"]');
      const tokenSubmitBtn = page.locator('.token-dialog button');
      await tokenInput.fill('test-token');
      await tokenSubmitBtn.click();
      await tokenDialog.waitFor({ state: 'hidden', timeout: 15000 });
      await page.waitForTimeout(1000);
    }

    // Wait for main app sidebar (org-rail) to be visible
    await page.waitForSelector('.org-rail', { timeout: 15000 });
    await screenshot.capture(page, 'gui-loaded');

    // Step 3: Open Join Party modal
    console.log('[TC-P-001] Step 3: Opening Join Party modal...');
    const joinPartyPage = new JoinPartyPage(page);
    await joinPartyPage.openModal();
    await screenshot.capture(page, 'join-party-modal-opened');

    // Step 4: Fill Join Party form
    console.log('[TC-P-001] Step 4: Filling Join Party form...');
    await joinPartyPage.fillForm({
      regUrl: TEST_DATA.regServerUrl,
      username: TEST_DATA.testUsername,
      inviteCode: TEST_DATA.validInviteCode
    });
    await screenshot.capture(page, 'join-party-form-filled');

    // Step 5: Submit Join Party
    console.log('[TC-P-001] Step 5: Submitting Join Party...');
    await joinPartyPage.submit();

    // Wait for success or error
    try {
      await joinPartyPage.waitForSuccess(20000);
      await screenshot.capture(page, 'join-party-success');
      console.log('[TC-P-001] Join Party succeeded');
    } catch (error) {
      await screenshot.capture(page, 'join-party-error');
      const errorText = await joinPartyPage.getErrorText();
      console.error('[TC-P-001] Join Party failed:', errorText);
      throw new Error(`Join Party failed: ${errorText}`);
    }

    // Wait for modal to close
    await page.waitForTimeout(2000);

    // Step 6: Navigate to zAgents panel
    console.log('[TC-P-001] Step 6: Navigating to zAgents panel...');
    const agentPage = new AgentPage(page);
    await agentPage.navigateToAgentsPanel();
    await page.waitForTimeout(1000);
    await screenshot.capture(page, 'zagents-panel-opened');

    // Step 7: Create AI Agent
    console.log('[TC-P-001] Step 7: Creating AI Agent...');
    await agentPage.createAgent(TEST_DATA.agentName);
    await page.waitForTimeout(2000);

    // Wait for agent to appear in list
    await agentPage.waitForAgentInList(TEST_DATA.agentName, 15000);
    await screenshot.capture(page, 'agent-created');

    // Step 8: Click on the agent to open chat
    console.log('[TC-P-001] Step 8: Opening chat with agent...');
    await agentPage.clickAgent(TEST_DATA.agentName);
    await page.waitForTimeout(2000);

    const chatPage = new ChatPage(page);
    await chatPage.waitForChatLoad();
    await screenshot.capture(page, 'chat-opened');

    // Step 9: Send message and wait for reply
    console.log('[TC-P-001] Step 9: Sending message...');
    await chatPage.sendMessage(TEST_DATA.testMessage);
    await screenshot.capture(page, 'message-sent');

    console.log('[TC-P-001] Step 9: Waiting for AI reply...');
    await chatPage.waitForReply(30000);
    await screenshot.capture(page, 'reply-received');

    // Verify reply exists
    const messageCount = await chatPage.getMessageCount();
    expect(messageCount).toBeGreaterThanOrEqual(2);

    console.log('[TC-P-001] Test completed successfully');
  });
});
