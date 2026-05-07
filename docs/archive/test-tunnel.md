# Tunnel App Test Plan

## Overview

Tunnel App is ClawParty zero-trust tunnel application, eliminating physical distance limitations, allowing device access from any location. This document records Tunnel App test plans and test cases.

## Test Scope

### 1. Basic Function Tests
- CLI help command
- Tunnel type support (TCP/UDP)
- Tunnel endpoint types (inbound/outbound)

### 2. TCP Tunnel Tests
- Create outbound endpoint
- Create inbound endpoint
- Tunnel connection
- Load balancing

### 3. UDP Tunnel Tests
- Create UDP tunnel
- UDP data transmission

### 4. Tunnel Management Tests
- List tunnels
- View tunnel details
- Delete tunnels

### 5. Multi-target Tests
- Single-target tunnel
- Multi-target load balancing

### 6. Network Environment Tests
- Cross-network connection
- Firewall traversal

---

## Test Cases

### TC-TN-001: CLI Help Commands

**Precondition**: Tunnel App is running

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm tunnel help` | Display help info |
| 2 | Execute `ztm tunnel get --help` | Display get command help |
| 3 | Execute `ztm tunnel open --help` | Display open command help |
| 4 | Execute `ztm tunnel close --help` | Display close command help |

---

### TC-TN-002: Create TCP Outbound Endpoint

**Precondition**: Accessible target service exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm tunnel open outbound tcp/greeting --targets 192.168.1.100:8080` | Outbound endpoint created successfully |
| 2 | Execute `ztm tunnel get outbound` | Display created endpoint |
| 3 | Verify endpoint info | Contains name and target address |
| 4 | Create same name endpoint again | Returns error or overwrite |

---

### TC-TN-003: Create TCP Inbound Endpoint

**Precondition**: Outbound endpoint exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm tunnel open inbound tcp/greeting --listen 18080` | Inbound endpoint created successfully |
| 2 | Execute `ztm tunnel get inbound` | Display created endpoint |
| 3 | Verify listening port | Port 18080 is being listened |
| 4 | Verify endpoint connection | Tunnel established successfully |

---

### TC-TN-004: TCP Tunnel Connection Test

**Precondition**: Both outbound and inbound endpoints created

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `curl localhost:18080` | Receive target service response |
| 2 | Request tunnel multiple times | Each time receives response |
| 3 | Verify response content | Content matches target service |
| 4 | Check tunnel status | Tunnel is active |

---

### TC-TN-005: Multi-target Load Balancing

**Precondition**: Multiple target services exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Create multi-target outbound: `ztm tunnel open outbound tcp/greeting --targets 192.168.1.100:8080 --targets 192.168.1.100:8081` | Outbound endpoint created successfully |
| 2 | Connect to inbound endpoint | Connection successful |
| 3 | Request multiple times consecutively | Requests distributed to different targets |
| 4 | Verify response alternation | Two targets respond alternately |

---

### TC-TN-006: View Tunnel Details

**Precondition**: Tunnel endpoint exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm tunnel describe outbound tcp/greeting` | Display detailed info |
| 2 | Verify contains target address | Target list correct |
| 3 | Execute `ztm tunnel describe inbound tcp/greeting` | Display inbound detailed info |
| 4 | Verify contains listening port | Listening port correct |

---

### TC-TN-007: Delete Tunnel Endpoint

**Precondition**: Tunnel endpoint exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm tunnel close outbound tcp/greeting` | Outbound endpoint deleted successfully |
| 2 | Execute `ztm tunnel get outbound` | Endpoint no longer displayed |
| 3 | Execute `ztm tunnel close inbound tcp/greeting` | Inbound endpoint deleted successfully |
| 4 | Execute `ztm tunnel get inbound` | Endpoint no longer displayed |

---

### TC-TN-008: UDP Tunnel Test

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Create UDP outbound: `ztm tunnel open outbound udp/dns --targets 8.8.8.8:53` | UDP outbound created successfully |
| 2 | Create UDP inbound: `ztm tunnel open inbound udp/dns --listen 18053` | UDP inbound created successfully |
| 3 | Send UDP data to local port | Data forwarded |
| 4 | Verify UDP response | Correct response received |

---

### TC-TN-009: Cross-network Tunnel Connection

**Precondition**: Two endpoints on different networks

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Create outbound on endpoint in network A | Created successfully |
| 2 | Create inbound on endpoint in network B | Created successfully |
| 3 | Access service through inbound endpoint | Access successful |
| 4 | Verify data integrity | Data transmitted correctly |

---

### TC-TN-010: Error Handling

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Access non-existent tunnel | Returns appropriate error message |
| 2 | Create inbound with port conflict | Returns port occupied error |
| 3 | Connect to unreachable target | Returns connection error |
| 4 | Delete tunnel in use | Warning or force delete |

---

## Test Environment Requirements

### Hardware Requirements
- At least 2 endpoints (simulating distributed environment)
- Different network environments (optional)

### Software Requirements
- ClawParty runtime
- ZTM network connection
- Test tools (curl, netcat)

### Test Data
- TCP service (port 8080, 8081)
- UDP service (port 53)
- Test scripts

---

## Test Execution Guide

### Unit Tests
- Test CLI command parsing
- Verify endpoint creation logic
- Test port allocation

### Integration Tests
- Test complete outbound to inbound flow
- Test multi-endpoint collaboration
- Test network exception handling

### End-to-End Tests
- Complete tunnel connection flow
- Cross-network scenarios
- High load scenarios

---

## Known Issues and Notes

1. **Port range**: Inbound endpoint needs available local port
2. **Firewall**: Ensure endpoints can communicate
3. **Load balancing**: Multi-target uses round-robin algorithm
4. **Protocol support**: Supports TCP and UDP
5. **Connection timeout**: Default timeout needs adjustment based on network environment