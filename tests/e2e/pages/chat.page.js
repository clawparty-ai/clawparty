/**
 * Page Object for the Chat area.
 *
 * GUI structure (ChatMain.vue + MessageInput.vue):
 * - Chat area: .chat-main
 * - Message input: .editor-content textarea
 * - Send button: .toolbar-btn.send-btn
 * - Message bubbles: .message-bubble
 * - Typing indicator: .typing-indicator
 */
export class ChatPage {
  constructor(page) {
    this.page = page;

    // Chat main area
    this.chatArea = page.locator('.chat-main');

    // Message input area (MessageInput.vue)
    this.messageInput = page.locator('.editor-content textarea');

    // Send button in toolbar
    this.sendBtn = page.locator('.toolbar-btn.send-btn');

    // Message bubbles
    this.userMessages = page.locator('.message.sent .message-bubble');
    this.allMessages = page.locator('.message-bubble');

    // Typing indicator
    this.typingIndicator = page.locator('.typing-indicator');
  }

  async sendMessage(text) {
    await this.messageInput.waitFor({ state: 'visible', timeout: 10000 });
    await this.messageInput.click();
    await this.messageInput.fill(text);
    // Press Enter to send (Shift+Enter not pressed)
    await this.messageInput.press('Enter');
  }

  async waitForReply(timeout = 30000) {
    // Wait for typing indicator to appear then disappear
    try {
      await this.typingIndicator.waitFor({ state: 'visible', timeout: 5000 });
      await this.typingIndicator.waitFor({ state: 'hidden', timeout });
    } catch (e) {
      // Typing indicator may not appear if reply is fast
    }

    // Wait until there are at least 2 message bubbles (user msg + reply)
    await this.page.waitForFunction(
      () => {
        const bubbles = document.querySelectorAll('.message-bubble');
        return bubbles.length >= 2;
      },
      { timeout }
    );
  }

  async getLastMessage() {
    const count = await this.allMessages.count();
    if (count === 0) return null;
    return this.allMessages.last().textContent();
  }

  async getMessageCount() {
    return this.allMessages.count();
  }

  async isInputVisible() {
    return this.messageInput.isVisible();
  }

  async isChatAreaVisible() {
    return this.chatArea.isVisible();
  }

  async waitForChatLoad(timeout = 10000) {
    await this.chatArea.waitFor({ state: 'visible', timeout });
    await this.messageInput.waitFor({ state: 'visible', timeout });
  }
}
