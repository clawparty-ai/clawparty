/**
 * API validation helper for Playwright tests.
 *
 * Provides utilities to intercept and validate API requests/responses.
 */
export class ApiHelper {
  constructor(page) {
    this.page = page;
    this.requests = [];
    this.responses = [];

    // Setup request/response listeners
    page.on('request', (request) => {
      this.requests.push({
        url: request.url(),
        method: request.method(),
        headers: request.headers(),
        postData: request.postData()
      });
    });

    page.on('response', (response) => {
      this.responses.push({
        url: response.url(),
        status: response.status(),
        headers: response.headers()
      });
    });
  }

  clearHistory() {
    this.requests = [];
    this.responses = [];
  }

  findRequest(urlPattern) {
    return this.requests.find(req => req.url.includes(urlPattern));
  }

  findResponse(urlPattern) {
    return this.responses.find(res => res.url.includes(urlPattern));
  }

  async waitForResponse(urlPattern, timeout = 10000) {
    return this.page.waitForResponse(
      response => response.url().includes(urlPattern),
      { timeout }
    );
  }

  async waitForRequest(urlPattern, timeout = 10000) {
    return this.page.waitForRequest(
      request => request.url().includes(urlPattern),
      { timeout }
    );
  }

  async getResponseBody(response) {
    try {
      return await response.json();
    } catch (e) {
      return await response.text();
    }
  }

  async validateJoinPartyRequest(expectedData) {
    const request = this.findRequest('/api/join') || this.findRequest('join');
    if (!request) {
      throw new Error('Join Party request not found');
    }

    const postData = JSON.parse(request.postData || '{}');

    if (expectedData.inviteCode && postData.inviteCode !== expectedData.inviteCode) {
      throw new Error(`Expected inviteCode ${expectedData.inviteCode}, got ${postData.inviteCode}`);
    }

    if (expectedData.username && postData.username !== expectedData.username) {
      throw new Error(`Expected username ${expectedData.username}, got ${postData.username}`);
    }

    return true;
  }

  async validateJoinPartyResponse(expectedStatus) {
    const response = this.findResponse('/api/join') || this.findResponse('join');
    if (!response) {
      throw new Error('Join Party response not found');
    }

    if (response.status !== expectedStatus) {
      throw new Error(`Expected status ${expectedStatus}, got ${response.status}`);
    }

    return true;
  }
}
