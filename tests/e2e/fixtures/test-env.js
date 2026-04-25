import { spawn, exec } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import { fileURLToPath } from 'url';

const execAsync = promisify(exec);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(__dirname, '../../..');

export class TestEnv {
  constructor() {
    this.ztmProcess = null;
    this.agentPort = 7784;
  }

  async startZtmAgent() {
    const ztmBin = process.env.ZTM_BIN || path.join(PROJECT_ROOT, 'bin/ztm');
    const dataDir = path.join(PROJECT_ROOT, 'tests/e2e/test-results/ztm-data');

    // Ensure any previous instance is fully dead
    if (this.ztmProcess) {
      await this.stopZtmAgent();
    }

    // Clean up data directory before starting to ensure fresh state
    console.log('[TestEnv] Cleaning up previous agent data...');
    try {
      await execAsync(`rm -rf "${dataDir}"`);
    } catch (error) {
      // Ignore if directory doesn't exist
    }

    // Guard: force-kill anything holding the port before start
    try {
      await execAsync(`lsof -ti:${this.agentPort} | xargs -r kill -9`);
    } catch {}
    await this.waitForPortRelease(this.agentPort, 3000);

    console.log('[TestEnv] Starting ZTM agent...');

    this.ztmProcess = spawn(ztmBin, ['run', 'agent', '--listen', `127.0.0.1:${this.agentPort}`, '--data', dataDir], {
      cwd: path.join(PROJECT_ROOT, 'agent'),  // Run from agent/ dir so GUI can be served
      stdio: 'pipe'
    });

    this.ztmProcess.stdout.on('data', (data) => {
      console.log(`[ZTM] ${data.toString().trim()}`);
    });

    this.ztmProcess.stderr.on('data', (data) => {
      console.error(`[ZTM Error] ${data.toString().trim()}`);
    });

    await this.waitForPort(this.agentPort, 15000);
    console.log('[TestEnv] ZTM agent started successfully');
  }

  async stopZtmAgent() {
    if (this.ztmProcess) {
      console.log('[TestEnv] Stopping ZTM agent...');
      const proc = this.ztmProcess;
      this.ztmProcess = null;

      await new Promise((resolve) => {
        let done = false;
        const finish = () => { if (!done) { done = true; resolve(); } };

        proc.on('exit', finish);
        proc.kill('SIGTERM');

        const killTimer = setTimeout(() => {
          if (!proc.killed) proc.kill('SIGKILL');
        }, 5000);

        proc.on('exit', () => clearTimeout(killTimer));
      });

      // Wait for port to actually be released
      await this.waitForPortRelease(this.agentPort, 3000);
      console.log('[TestEnv] ZTM agent stopped');
    }
  }

  async waitForPort(port, timeout = 10000) {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      try {
        const { stdout } = await execAsync(`lsof -ti:${port}`);
        if (stdout.trim()) {
          return true;
        }
      } catch (error) {
        // Port not ready yet
      }
      await new Promise(resolve => setTimeout(resolve, 500));
    }

    throw new Error(`Port ${port} did not become available within ${timeout}ms`);
  }

  async waitForPortRelease(port, timeout = 3000) {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      const inUse = await this.checkPort(port);
      if (!inUse) return;
      await new Promise(r => setTimeout(r, 200));
    }
  }

  async checkPort(port) {
    try {
      const { stdout } = await execAsync(`lsof -ti:${port}`);
      return !!stdout.trim();
    } catch (error) {
      return false;
    }
  }
}
