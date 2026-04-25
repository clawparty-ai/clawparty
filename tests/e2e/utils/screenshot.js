import fs from 'fs';
import path from 'path';

export class ScreenshotHelper {
  constructor(testName) {
    this.testName = testName;
    this.screenshotDir = path.join(process.cwd(), 'screenshots', testName);
    this.counter = 0;

    // Create screenshot directory
    if (!fs.existsSync(this.screenshotDir)) {
      fs.mkdirSync(this.screenshotDir, { recursive: true });
    }
  }

  async capture(page, stepName) {
    this.counter++;
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const filename = `${this.counter.toString().padStart(2, '0')}-${timestamp}-${stepName}.png`;
    const filepath = path.join(this.screenshotDir, filename);

    await page.screenshot({ path: filepath, fullPage: true });
    console.log(`[Screenshot] ${filepath}`);

    return filepath;
  }

  getScreenshotPath(stepName) {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const filename = `${this.counter.toString().padStart(2, '0')}-${timestamp}-${stepName}.png`;
    return path.join(this.screenshotDir, filename);
  }
}
