/**
 * Page Object for zAgent management in the sidebar.
 *
 * GUI structure (ChatSidebar.vue):
 * - zAgents rail icon: .org-icon[title="zAgents"]
 * - Create button: .add-agent-btn in zAgents panel header
 * - zAgent list items: .zagent-item
 */
export class AgentPage {
  constructor(page) {
    this.page = page;

    // Org rail icon for zAgents section
    this.zagentsOrgIcon = page.locator('.org-icon[title="zAgents"]');

    // Panel header with "+A" create button
    this.createAgentBtn = page.locator('.add-agent-btn');

    // Create zAgent dialog
    this.createDialog = page.locator('.zagent-create-dialog');
    this.agentNameInput = this.createDialog.locator('.search-input').first();
    this.createSubmitBtn = this.createDialog.locator('.modal-create-btn');
    this.createCancelBtn = this.createDialog.locator('.modal-cancel-btn');

    // Agent list in sidebar panel
    this.agentList = page.locator('.sidebar-panel .zagent-item');
  }

  async navigateToAgentsPanel() {
    // Wait for Vue app sidebar to render, then switch to zAgents panel
    await this.page.waitForSelector('.org-rail', { timeout: 15000 });
    await this.zagentsOrgIcon.waitFor({ state: 'visible', timeout: 5000 });
    await this.zagentsOrgIcon.click();
    await this.page.locator('.panel-title').getByText('zAgents').waitFor({ state: 'visible', timeout: 5000 });
  }

  async openCreateDialog() {
    await this.createAgentBtn.waitFor({ state: 'visible', timeout: 5000 });
    await this.createAgentBtn.click();
    await this.createDialog.waitFor({ state: 'visible', timeout: 5000 });
  }

  async fillAgentName(name) {
    await this.agentNameInput.fill(name);
  }

  async submitCreateAgent() {
    await this.createSubmitBtn.click();
  }

  async createAgent(name) {
    await this.openCreateDialog();
    await this.fillAgentName(name);
    await this.submitCreateAgent();
    await this.createDialog.waitFor({ state: 'hidden', timeout: 10000 });
  }

  async isAgentInList(name) {
    const agents = this.page.locator('.sidebar-panel').filter({ hasText: name });
    return agents.count().then(c => c > 0);
  }

  async clickAgent(name) {
    await this.page.locator('.sidebar-panel').getByText(name).first().click();
  }

  async waitForAgentInList(name, timeout = 10000) {
    await this.page.locator('.sidebar-panel').getByText(name).waitFor({ state: 'visible', timeout });
  }
}
