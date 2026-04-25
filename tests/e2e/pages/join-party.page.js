/**
 * Page Object for the Join Party modal dialog.
 *
 * The modal is triggered by clicking the 🌐 button (title="加入组织") in the org rail.
 * It is rendered via Teleport to <body>, so selectors are relative to the page root.
 *
 * GUI structure (ChatSidebar.vue):
 *   - Trigger: button.new-group-rail-btn[title="加入组织"] (first rail button)
 *   - Modal: .modal-backdrop containing .modal-dialog with title "加入组织"
 *   - Fields: .join-party-body inputs in order: URL, username, invite code
 */
export class JoinPartyPage {
  constructor(page) {
    this.page = page;

    // Trigger button: the first .new-group-rail-btn has title="加入组织" (globe icon)
    this.joinPartyRailBtn = page.locator('button.new-group-rail-btn[title="加入组织"]');

    // Modal: Teleported to body; identify by "加入组织" title text to avoid collision
    this.modal = page.locator('.modal-backdrop').filter({ hasText: '加入组织' }).locator('.modal-dialog');

    // Form fields inside .join-party-body (order: URL, username, invite code)
    this.regUrlInput = page.locator('.join-party-body input').nth(0);
    this.usernameInput = page.locator('.join-party-body input').nth(1);
    this.inviteCodeInput = page.locator('.join-party-body input').nth(2);

    // Footer buttons inside the join-party modal
    this.cancelBtn = this.modal.locator('.modal-cancel-btn');
    this.submitBtn = this.modal.locator('.modal-create-btn');

    // Status messages
    this.stepText = page.locator('.join-party-step');
    this.errorText = page.locator('.join-party-error');
    this.successText = page.locator('.join-party-success');
  }

  async openModal() {
    // Wait for Vue app to be mounted (sidebar rendered)
    await this.page.waitForSelector('.org-rail', { timeout: 15000 });
    await this.joinPartyRailBtn.waitFor({ state: 'visible', timeout: 5000 });
    await this.joinPartyRailBtn.click();
    await this.modal.waitFor({ state: 'visible', timeout: 5000 });
  }

  async closeModal() {
    await this.cancelBtn.click();
    await this.modal.waitFor({ state: 'hidden', timeout: 5000 });
  }

  async fillForm({ regUrl, username, inviteCode }) {
    if (regUrl !== undefined) {
      await this.regUrlInput.fill(regUrl);
    }
    if (username !== undefined) {
      await this.usernameInput.fill(username);
    }
    if (inviteCode !== undefined) {
      await this.inviteCodeInput.fill(inviteCode);
    }
  }

  async submit() {
    await this.submitBtn.click();
  }

  async fillAndSubmit({ regUrl, username, inviteCode }) {
    await this.fillForm({ regUrl, username, inviteCode });
    await this.submit();
  }

  async isModalVisible() {
    return this.modal.isVisible();
  }

  async isSubmitButtonEnabled() {
    return !await this.submitBtn.isDisabled();
  }

  async getErrorText() {
    return this.errorText.textContent();
  }

  async getSuccessText() {
    return this.successText.textContent();
  }

  async waitForSuccess(timeout = 20000) {
    await this.successText.waitFor({ state: 'visible', timeout });
  }

  async waitForError(timeout = 20000) {
    await this.errorText.waitFor({ state: 'visible', timeout });
  }

  async waitForLoading(timeout = 5000) {
    await this.page.waitForFunction(
      () => document.querySelector('.modal-create-btn')?.textContent?.includes('加入中'),
      { timeout }
    );
  }
}
