/**
 * ClawParty WebSocket 客户端（修复版）
 * 直接替换现有的 zAgentWS 类。
 *
 * 修复的问题：
 *  1. 无限重连 + 指数退避（封顶 30s + 随机抖动），不再 3~5 次后永久放弃
 *  2. 应用层心跳：每 25s 发 ping，连续 2 次无响应主动断开重连（解决半开连接）
 *  3. 关闭码 1000 也重连（服务端重启/发版/nginx reload 不再导致永久断线）
 *  4. 连接状态回调：UI 可显示"已断开，正在重连（第 N 次）"
 *  5. 重连成功后触发 onReconnected，用于增量补拉断开期间的消息
 *
 * 用法与原类基本一致：
 *   const ws = new ZAgentWS(agentName, sessionId, {
 *     onMessage: (msg) => { ... },
 *     onStateChange: (state, info) => { ... },  // 新增：驱动断线状态条
 *     onReconnected: () => { ... },             // 新增：补拉缺失消息
 *   });
 *   ws.connect();
 *   ws.send(payload);
 *   ws.destroy();  // 用户主动登出/切换频道时调用，不会再重连
 */

export class ZAgentWS {
  /**
   * @param {string} agentName
   * @param {string} sessionId
   * @param {object} handlers
   * @param {(data: any) => void} [handlers.onMessage]
   * @param {() => void} [handlers.onOpen]
   * @param {(state: string, info: {attempt: number, nextRetryMs: number|null}) => void} [handlers.onStateChange]
   *        state: 'connecting' | 'connected' | 'reconnecting' | 'destroyed'
   * @param {() => void} [handlers.onReconnected]  重连成功（非首次连接）时触发，用于补拉消息
   */
  constructor(agentName, sessionId, handlers = {}) {
    this.agentName = agentName;
    this.sessionId = sessionId;
    this.onMessage = handlers.onMessage || null;
    this.onOpen = handlers.onOpen || null;
    this.onStateChange = handlers.onStateChange || null;
    this.onReconnected = handlers.onReconnected || null;

    this.ws = null;
    this.destroyed = false;       // 主动销毁后不再重连
    this.everConnected = false;   // 区分"首次连接"和"重连成功"

    // ---- 重连参数 ----
    this.reconnectAttempts = 0;
    this.baseDelayMs = 1000;      // 退避基数 1s
    this.maxDelayMs = 30000;      // 封顶 30s（不再永久放弃）
    this.reconnectTimer = null;

    // ---- 心跳参数 ----
    this.heartbeatIntervalMs = 25000;  // 每 25s 一次心跳（小于常见 NAT/代理 60s 空闲超时）
    this.heartbeatTimer = null;
    this.missedHeartbeats = 0;
    this.maxMissedHeartbeats = 2;      // 连续 2 次无响应判定连接死亡
  }

  buildUrl() {
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    return `${proto}//${host}/ws/chat?agent=${encodeURIComponent(this.agentName)}&session_id=${encodeURIComponent(this.sessionId)}`;
  }

  connect() {
    if (this.destroyed) return;
    this._clearReconnectTimer();
    this._emitState(this.everConnected ? 'reconnecting' : 'connecting');

    let ws;
    try {
      ws = new WebSocket(this.buildUrl(), 'zeroclaw.v1');
    } catch (err) {
      console.error('[zAgentWS] 创建连接失败:', err);
      this._scheduleReconnect();
      return;
    }
    this.ws = ws;

    ws.onopen = () => {
      console.log('[zAgentWS] 已连接');
      const wasReconnect = this.everConnected;
      this.everConnected = true;
      this.reconnectAttempts = 0;
      this.missedHeartbeats = 0;
      this._startHeartbeat();
      this._emitState('connected');
      this.onOpen?.();
      if (wasReconnect) {
        // 重连成功：补拉断开期间缺失的消息
        this.onReconnected?.();
      }
    };

    ws.onmessage = (evt) => {
      // 任何下行消息都视为"连接活着"的证据，重置心跳计数
      this.missedHeartbeats = 0;
      if (typeof evt.data !== 'string') return;
      // 服务端心跳应答不抛给业务层
      let data;
      try { data = JSON.parse(evt.data); } catch { return; }
      if (data && data.type === 'pong') return;
      this.onMessage?.(data);
    };

    ws.onerror = (evt) => {
      console.error('[zAgentWS] 连接错误:', evt);
      // 不在这里重连，onclose 一定会随后触发，统一在 onclose 处理
    };

    ws.onclose = (evt) => {
      console.log('[zAgentWS] 已断开:', evt.code, evt.reason || '');
      this._stopHeartbeat();
      this.ws = null;
      if (this.destroyed) return;
      // 修复点：1000 也重连。服务端重启/发版/nginx reload 通常发 1000 或 1001，
      // 旧逻辑在这里直接 return，导致永久断线。
      this._scheduleReconnect();
    };
  }

  send(payload) {
    const data = typeof payload === 'string' ? payload : JSON.stringify(payload);
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(data);
      return true;
    }
    console.warn('[zAgentWS] 未连接，消息未发送');
    return false;  // 调用方应据此提示用户，而不是让消息静默进列表
  }

  /** 主动销毁（登出/切换频道）。之后不再重连。 */
  destroy() {
    this.destroyed = true;
    this._clearReconnectTimer();
    this._stopHeartbeat();
    if (this.ws) {
      const ws = this.ws;
      this.ws = null;
      // 服务端不回 close 帧时浏览器会卡在 CLOSING，这里兜底强制清理
      try { ws.close(1000, 'client-destroy'); } catch { /* ignore */ }
      setTimeout(() => {
        if (ws.readyState !== WebSocket.CLOSED) {
          ws.onopen = ws.onmessage = ws.onerror = ws.onclose = null;
        }
      }, 3000);
    }
    this._emitState('destroyed');
  }

  // ---------------- 内部实现 ----------------

  _scheduleReconnect() {
    if (this.destroyed) return;
    this._clearReconnectTimer();
    this.reconnectAttempts += 1;
    // 指数退避：1s, 2s, 4s, 8s ... 封顶 30s，加 ±25% 抖动避免雪崩
    const exp = Math.min(this.baseDelayMs * 2 ** (this.reconnectAttempts - 1), this.maxDelayMs);
    const jitter = exp * (0.75 + Math.random() * 0.5);
    const delay = Math.min(Math.round(jitter), this.maxDelayMs);
    console.log(`[zAgentWS] 第 ${this.reconnectAttempts} 次重连，${delay}ms 后重试`);
    this._emitState('reconnecting', { nextRetryMs: delay });
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }

  _startHeartbeat() {
    this._stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
      this.missedHeartbeats += 1;
      if (this.missedHeartbeats > this.maxMissedHeartbeats) {
        // 连续无响应：半开连接，主动断开触发重连
        console.warn('[zAgentWS] 心跳超时，判定连接死亡，主动重连');
        const ws = this.ws;
        this.ws = null;
        // 清掉回调再 close，避免 onclose 里重复调度（close 事件可能不来，直接调度）
        ws.onopen = ws.onmessage = ws.onerror = ws.onclose = null;
        try { ws.close(4000, 'heartbeat-timeout'); } catch { /* ignore */ }
        this._stopHeartbeat();
        this._scheduleReconnect();
        return;
      }
      try { this.ws.send(JSON.stringify({ type: 'ping', ts: Date.now() })); } catch { /* ignore */ }
    }, this.heartbeatIntervalMs);
  }

  _stopHeartbeat() {
    if (this.heartbeatTimer) { clearInterval(this.heartbeatTimer); this.heartbeatTimer = null; }
    this.missedHeartbeats = 0;
  }

  _clearReconnectTimer() {
    if (this.reconnectTimer) { clearTimeout(this.reconnectTimer); this.reconnectTimer = null; }
  }

  _emitState(state, extra = {}) {
    this.onStateChange?.(state, { attempt: this.reconnectAttempts, nextRetryMs: null, ...extra });
  }
}
