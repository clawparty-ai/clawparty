# Proxy App Test Plan

## Overview

Proxy App is ClawParty zero-trust proxy application, allowing device access from any location. This document records Proxy App test plans and test cases.

## Test Scope

### 1. Basic Function Tests
- CLI help commands
- Proxy configuration
- Listening and forwarding endpoints

### 2. Listening Endpoint Configuration Tests
- Set listening address
- Modify listening port
- Close listening

### 3. Forwarding Endpoint Configuration Tests
- Add forwarding targets
- Remove forwarding targets
- View forwarding rules

### 4. Proxy Connection Tests
- HTTP proxy
- HTTPS proxy
- SOCKS proxy (optional)

### 5. Domain and IP Filtering Tests
- Domain matching
- IP range matching
- Wildcard support

### 6. Multi-endpoint Tests
- Multiple forwarding endpoints
- Multiple listening endpoints

---

## Test Cases

### TC-PX-001: CLI Help Commands

**Precondition**: Proxy App is running

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy help` | Display help info |
| 2 | Execute `ztm proxy help config` | Display config command help |
| 3 | Verify option description | Display --set-listen, --add-target, --remove-target |

---

### TC-PX-002: Configure Forwarding Endpoint

**Precondition**: Two endpoints exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --add-target 0.0.0.0/0 '*'` | Config successful |
| 2 | Execute `ztm proxy config` | Display config info |
| 3 | Verify target list | Contains 0.0.0.0/0 and * |
| 4 | Verify listening status | Display not listening |

---

### TC-PX-003: Configure Listening Endpoint

**Precondition**: Two endpoints exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --set-listen 0.0.0.0:1080` | Listening config successful |
| 2 | Execute `ztm proxy config` | Display listening address |
| 3 | Verify listening port | Port 1080 is being listened |
| 4 | Verify proxy status | Display listening |

---

### TC-PX-004: HTTP Proxy Test

**Precondition**: Both listening and forwarding endpoints configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `curl --proxy http://localhost:1080 http://example.com` | Proxy request successful |
| 2 | Verify response content | Content matches direct access |
| 3 | Execute `curl --proxy http://localhost:1080 http://internal-host:8080` | Access internal service successfully |
| 4 | Verify forwarding rules match | Request forwarded correctly |

---

### TC-PX-005: Add Specific Domain Target

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --add-target '*.example.com'` | Added successfully |
| 2 | Execute `ztm proxy config` | Target list contains *.example.com |
| 3 | Proxy access example.com | Proxy successful |
| 4 | Proxy access other.com | Proxy failed (not in target list) |

---

### TC-PX-006: Add IP Range Target

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --add-target '192.168.0.0/16'` | Added successfully |
| 2 | Execute `ztm proxy config` | Target list contains IP range |
| 3 | Proxy access 192.168.1.100 | Proxy successful |
| 4 | Proxy access 10.0.0.1 | Proxy failed (not in range) |

---

### TC-PX-007: Remove Forwarding Target

**Precondition**: Configured forwarding target exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --remove-target '*.example.com'` | Removed successfully |
| 2 | Execute `ztm proxy config` | Target list no longer contains that domain |
| 3 | Proxy access example.com | Proxy failed |
| 4 | Verify other targets unaffected | Other targets still proxyable |

---

### TC-PX-008: Modify Listening Port

**Precondition**: Listening configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --set-listen 0.0.0.0:1081` | Modified successfully |
| 2 | Verify old port | Port 1080 no longer listening |
| 3 | Verify new port | Port 1081 starts listening |
| 4 | Proxy through new port | Proxy successful |

---

### TC-PX-009: Close Proxy Listening

**Precondition**: Listening configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm proxy config --set-listen` | Close listening |
| 2 | Execute `ztm proxy config` | Display not listening |
| 3 | Try proxy access | Connection failed |
| 4 | Verify forwarding config retained | Forwarding targets still exist |

---

### TC-PX-010: Domain Resolution Test

**Precondition**: Forwarding endpoint has domain resolution

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Configure hosts on forwarding endpoint | Configure specific domain resolution |
| 2 | Access domain through proxy | Use forwarding endpoint's resolution |
| 3 | Verify access successful | Service responds correctly |
| 4 | Verify DNS resolution location | Use forwarding endpoint's DNS |

---

### TC-PX-011: Multi-endpoint Proxy

**Precondition**: Multiple endpoints exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Configure endpoint A as forwarding endpoint | Config successful |
| 2 | Configure endpoint B as listening endpoint | Config successful |
| 3 | Proxy access through endpoint B | Request forwarded to endpoint A |
| 4 | Endpoint A forwards to target | Complete proxy chain works |

---

### TC-PX-012: HTTPS Proxy Test

**Precondition**: Both listening and forwarding endpoints configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `curl --proxy http://localhost:1080 https://example.com` | HTTPS proxy successful |
| 2 | Verify certificate validation | Verify target certificate correctly |
| 3 | Verify encrypted channel | Data transmitted encrypted |
| 4 | Verify CONNECT method | Use HTTP CONNECT |

---

### TC-PX-013: Error Handling

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Configure listening with port conflict | Returns port occupied error |
| 2 | Proxy to unreachable target | Returns connection error |
| 3 | Proxy to non-matching target | Returns target unreachable error |
| 4 | Invalid config parameters | Returns parameter error |

---

## Test Environment Requirements

### Hardware Requirements
- At least 2 endpoints (simulating distributed proxy environment)
- Different network environments (optional)

### Software Requirements
- ClawParty runtime
- ZTM network connection
- Test tools (curl, wget)

### Test Data
- HTTP/HTTPS services
- Domain and IP range test cases
- Network configuration

---

## Test Execution Guide

### Unit Tests
- Test CLI command parsing
- Verify config storage logic
- Test target matching algorithm

### Integration Tests
- Test complete listening to forwarding flow
- Test multi-endpoint collaboration
- Test config change sync

### End-to-end Tests
- Complete proxy request flow
- Multi-hop proxy scenarios
- High concurrency scenarios

---

## Known Issues and Notes

1. **Port requirement**: Listening endpoint needs available local port
2. **Firewall**: Ensure endpoints can communicate
3. **DNS resolution**: Use forwarding endpoint's DNS config
4. **Protocol support**: Support HTTP/HTTPS proxy
5. **Target matching**: Support domain, IP range and wildcard