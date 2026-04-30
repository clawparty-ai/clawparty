// Test: verify agent status API is fast after switching from lsof to kill-0
const http = require('http');
const https = require('https');

const TOKEN = 'enjoy-party';
const BASE = 'http://127.0.0.1:6789';

function fetch(path) {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const req = http.get(`${BASE}${path}?token=${TOKEN}`, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        resolve({ status: res.statusCode, data, time: Date.now() - start });
      });
    });
    req.setTimeout(5000, () => reject(new Error('Timeout')));
    req.on('error', reject);
  });
}

async function test() {
  console.log(`[${new Date().toISOString()}] Starting ztm agent API performance test...`);

  // 1. Test /api/agents (this used to call lsof for each agent)
  console.log('\n--- Test 1: GET /api/agents (fast path with cache) ---');
  for (let i = 0; i < 5; i++) {
    const r = await fetch('/api/agents');
    console.log(`  Run ${i + 1}: ${r.time}ms, status=${r.status}`);
    if (r.time > 3000) {
      console.error('  ⚠️ WARNING: Slow response detected!');
    }
  }

  // 2. Test /api/agents/{name}/status (individual agent)
  console.log('\n--- Test 2: GET /api/agents/0%23Agent/status ---');
  for (let i = 0; i < 5; i++) {
    const r = await fetch('/api/agents/0%23Agent/status');
    console.log(`  Run ${i + 1}: ${r.time}ms, status=${r.status}`);
    if (r.time > 1000) {
      console.error('  ⚠️ WARNING: Slow individual agent status!');
    }
  }

  // 3. Test /api/groupchats (group chat list)
  console.log('\n--- Test 3: GET /api/groupchats ---');
  for (let i = 0; i < 3; i++) {
    const r = await fetch('/api/groupchats');
    console.log(`  Run ${i + 1}: ${r.time}ms, status=${r.status}`);
  }

  // 4. Cache test: verify second call is faster (cached)
  console.log('\n--- Test 4: Cache verification (same agent, back-to-back) ---');
  const r1 = await fetch('/api/agents/0%23Agent/status');
  const r2 = await fetch('/api/agents/0%23Agent/status');
  console.log(`  First:  ${r1.time}ms`);
  console.log(`  Second: ${r2.time}ms (cached)`);
  if (r2.time >= r1.time && r1.time > 50) {
    console.log('  ⚠️ Cache may not be working (second call not faster)');
  } else {
    console.log('  ✅ Cache is effective');
  }

  // 5. Test WebSocket proxy to 0#Agent
  console.log('\n--- Test 5: WS /ws/chat (WebSocket proxy) ---');
  const wsStart = Date.now();
  try {
    const ws = new (require('ws'))(`ws://127.0.0.1:6789/ws/chat?agent=0%23Agent&session_id=me&token=${TOKEN}`);
    await new Promise((resolve, reject) => {
      ws.on('open', () => {
        console.log(`  Connected in ${Date.now() - wsStart}ms`);
        ws.send(JSON.stringify({ type: 'message', content: 'hello from test' }));
        setTimeout(() => {
          ws.close();
          resolve();
        }, 500);
      });
      ws.on('error', (e) => {
        console.log(`  WS error: ${e.message}`);
        reject(e);
      });
      setTimeout(() => reject(new Error('WS timeout')), 3000);
    });
    console.log('  ✅ WebSocket proxy working');
  } catch (e) {
    console.log(`  ⚠️ WebSocket test failed: ${e.message}`);
  }

  console.log('\n--- All tests completed ---');
}

test().catch(console.error);
