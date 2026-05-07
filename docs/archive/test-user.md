# User Management Test Plan

## Overview

User management is one of ClawParty core functions, including user registration, identity authentication, status management and permission control. This document records user management test plans and test cases.

## Test Scope

### 1. User Registration Tests
- Registration API availability
- Registration parameter validation
- Certificate issuance
- Permit generation

### 2. User Status Tests
- Status flow
- Status query
- Status change

### 3. User Ban Tests
- Ban user
- Unban user
- Ban effect

### 4. Identity Authentication Tests
- Certificate verification
- Connection establishment
- Permission check

### 5. Audit Log Tests
- API logs
- User event logs
- Log query

### 6. CLI Command Tests
- Join Mesh
- Configuration management
- User operations

---

## Test Cases

### TC-UR-001: Enable Registration API

**Precondition**: Hub is running

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm hub --enable-registration` | Registration API started |
| 2 | Verify registration port | Port 5678 is being listened |
| 3 | Access registration API | Returns API info |
| 4 | Verify plain HTTP | No client certificate required |

---

### TC-UR-002: User Registration Success

**Precondition**: Registration API enabled

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Get client public key | Returns RSA public key |
| 2 | Submit registration request | Request successful |
| 3 | Verify response format | Contains UserName, EpName, Permit |
| 4 | Verify user status | Status is permit-issued |
| 5 | Verify Permit content | Contains CA certificate and user certificate |

---

### TC-UR-003: Registration Parameter Validation

**Precondition**: Registration API enabled

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Submit request without UserName | Returns 400 error |
| 2 | Submit request without PublicKey | Returns 400 error |
| 3 | Submit invalid public key format | Returns 400 error |
| 4 | Submit complete valid request | Returns 201 success |

---

### TC-UR-004: Duplicate Registration Handling

**Precondition**: User already registered

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Register with same username | Returns 409 user already exists |
| 2 | Verify user status unchanged | Status remains unchanged |
| 3 | Verify log record | Records duplicate registration attempt |
| 4 | Register with different username | Returns 201 success |

---

### TC-UR-005: User Connection Activation

**Precondition**: User has obtained Permit

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Connect to Hub using Permit | Connection successful |
| 2 | Verify user status | Status changes to activated |
| 3 | Verify endpoint online | Endpoint appears in list |
| 4 | Verify connection log | Records connect event |

---

### TC-UR-006: View User Status

**Precondition**: Registered user exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Query user info | Returns user details |
| 2 | Verify status field | Displays correct status |
| 3 | Verify time fields | Contains creation and update time |
| 4 | Verify endpoint name | Displays correct endpoint name |

---

### TC-UR-007: Ban User

**Precondition**: User is activated

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Administrator bans user | Ban successful |
| 2 | Verify user status | Status changes to evicted |
| 3 | Verify connection disconnected | User connection disconnected |
| 4 | Verify ban log | Records evict event |
| 5 | Try to reconnect | Connection rejected |

---

### TC-UR-008: Unban User

**Precondition**: User has been banned

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Administrator unbans user | Unban successful |
| 2 | Verify user status | Status restored to activated |
| 3 | Verify ban record deleted | Record deleted from evictions table |
| 4 | Verify unban log | Records evict_removed event |
| 5 | User reconnects | Connection successful |

---

### TC-UR-009: Banned User Re-registration

**Precondition**: User has been banned

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Banned user tries to register | Returns 403 rejected |
| 2 | Verify registration log | Records banned user registration attempt |
| 3 | Try to register after unban | Still returns 409 (user already exists) |

---

### TC-UR-010: API Log Recording

**Precondition**: Registration API enabled

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute registration operation | Operation successful |
| 2 | Query API log | Log contains the operation |
| 3 | Verify log fields | Contains time, method, path, status |
| 4 | Verify client IP | Records correct IP |

---

### TC-UR-011: User Event Logs

**Precondition**: User operations executed

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute user registration | Log records cert_issued |
| 2 | User connects | Log records connect |
| 3 | User disconnects | Log records disconnect |
| 4 | Ban user | Log records evict |
| 5 | Unban user | Log records evict_removed |

---

### TC-UR-012: Join Mesh Using CLI

**Precondition**: Hub running, registration API enabled

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm join party` | Auto register and join |
| 2 | Verify Permit file | File saved |
| 3 | Verify user info | Info written to clawparty.md |
| 4 | Verify connection status | Successfully connected to Mesh |

---

### TC-UR-013: Manual Registration Using CLI

**Precondition**: Hub running, registration API enabled

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm try openclaw` | Registration successful |
| 2 | Verify parameter passing | All parameters passed correctly |
| 3 | Verify Permit saved | Saved to specified path |
| 4 | Verify connection established | Successfully connected to Mesh |

---

### TC-UR-014: Certificate Expiration Handling

**Precondition**: Certificate about to expire

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Wait for certificate to expire | Certificate expires |
| 2 | Try to connect | Connection failed |
| 3 | Re-register for new certificate | Need to delete old record |
| 4 | Obtain new certificate | Certificate validity reset |

---

### TC-UR-015: Multi-endpoint User

**Precondition**: User has existing endpoint

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Add new endpoint with same user | Returns 409 user already exists |
| 2 | Verify existing endpoint | Endpoint still online |
| 3 | Verify user status | Status remains activated |
| 4 | Verify log record | Records duplicate registration attempt |

---

## Test Environment Requirements

### Hardware Requirements
- Hub server
- At least 2 client endpoints

### Software Requirements
- ClawParty Hub
- ClawParty Agent
- SQLite database

### Test Data
- Test user account
- Test public key
- Test Permit

---

## Test Execution Guide

### Unit Tests
- Test registration parameter validation
- Verify status flow logic
- Test log recording

### Integration Tests
- Test complete registration to connection flow
- Test ban and unban flow
- Test audit logs

### End-to-End Tests
- Complete user lifecycle
- Multi-user concurrent registration
- Exception scenario handling

---

## Known Issues and Notes

1. **Registration API security**: Registration port is plain HTTP, needs network security measures
2. **Certificate validity**: Default 365 days, need to re-register after expiration
3. **Ban time**: Ban has timestamp and expiration time
4. **User records**: Need to delete user record to re-register
5. **Log retention**: Log table will continue to grow, needs periodic cleanup