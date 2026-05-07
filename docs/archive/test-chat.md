# Chat App Test Plan

## Overview

Chat App is ClawParty instant messaging application, supporting peer-to-peer chat, group chat, auto-reply, message filtering and more. This document records Chat App test plans and test cases.

## Test Scope

### 1. Basic Function Tests
- Endpoint management
- User list
- Chat list
- Message query

### 2. Peer Chat Tests
- Send messages
- Receive messages
- Message history
- Session management

### 3. Group Chat Tests
- Create group
- Update group members
- Modify group name
- Delete group
- Leave group
- Send group messages

### 4. Auto-reply Tests
- Peer auto-reply
- Group auto-reply
- Agent auto-reply
- Semi-automation mode

### 5. Message Filter Tests
- Credit system
- Keyword filtering
- Send filtering

### 6. File Upload Tests
- Chat file upload
- Session file upload
- File download

### 7. GUI Function Tests
- Chat interface
- Group management interface
- Auto-reply configuration interface

---

## Test Cases

### TC-CH-001: Get App Info

**Precondition**: Chat App is running

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/appinfo | Returns app name, username, endpoint info |
| 2 | Verify return fields | Contains name, provider, username, endpoint |

---

### TC-CH-002: Get Endpoints List

**Precondition**: Other endpoints in network

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/endpoints | Returns all discoverable endpoints |
| 2 | Verify endpoint info | Contains username and endpoint ID |

---

### TC-CH-003: Get Users List

**Precondition**: Other users in network

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/users | Returns all users list |
| 2 | Verify user uniqueness | Username deduped and sorted |

---

### TC-CH-004: Get Chats List

**Precondition**: Active chats exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/chats | Returns all chats list |
| 2 | Verify chat types | Distinguishes peer and group chats |
| 3 | Verify update time | Sorted by update time |

---

### TC-CH-005: Send Peer Message

**Precondition**: Communicable endpoint exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/peers/{peer}/messages | Message sent successfully, returns 201 |
| 2 | Message body contains text field | Message content parsed correctly |
| 3 | Verify message storage | Message appears in chat history |

---

### TC-CH-006: Receive Peer Message

**Precondition**: Other endpoint sent message

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/peers/{peer}/messages | Returns message list |
| 2 | Use since parameter filter | Returns only messages after timestamp |
| 3 | Verify message format | Contains time, sender, message fields |

---

### TC-CH-007: Create Group

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/groups/{creator}/{groupId} | Group created successfully, returns 201 |
| 2 | Request body contains name and members | Group info set correctly |
| 3 | Call GET /api/groups/{creator}/{groupId} | Returns group info |
| 4 | Verify group members | Member list contains creator and specified members |

---

### TC-CH-008: Update Group Members

**Precondition**: Group has been created

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/groups/{creator}/{groupId} | Update member list |
| 2 | Add new member | New member appears in member list |
| 3 | Remove member | Member removed from list |
| 4 | Verify update time | updateTime updated |

---

### TC-CH-009: Modify Group Name

**Precondition**: Group created, current user is creator

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/groups/{creator}/{groupId} | Update group name |
| 2 | Request body contains new name | Name updated successfully |
| 3 | Non-creator tries to modify | Returns 403 |

---

### TC-CH-010: Send Group Message

**Precondition**: Group created, user is member

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/groups/{creator}/{groupId}/messages | Message sent successfully, returns 201 |
| 2 | Message contains text field | Message content parsed correctly |
| 3 | Verify message storage | Message appears in group chat |

---

### TC-CH-011: Receive Group Message

**Precondition**: Messages exist in group

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/groups/{creator}/{groupId}/messages | Returns message list |
| 2 | Use since parameter filter | Returns only messages after timestamp |
| 3 | Verify sender info | Contains sender field |

---

### TC-CH-012: Delete Group (Creator)

**Precondition**: User is group creator

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call DELETE /api/groups/{creator}/{groupId} | Group deleted, returns 204 |
| 2 | Call GET again | Group does not exist, returns 404 |
| 3 | Verify cleanup | Group removed from chat list |

---

### TC-CH-013: Leave Group (Member)

**Precondition**: User is group member (not creator)

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call DELETE /api/groups/{creator}/{groupId}?leave=1 | Successfully left, returns 204 |
| 2 | Query group again | Group removed from local list |
| 3 | Other members unaffected | Group still exists |

---

### TC-CH-014: Peer Auto-reply Configuration

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/peers/{peer}/auto-reply | Returns current config |
| 2 | Call POST /api/peers/{peer}/auto-reply | Enable auto-reply |
| 3 | Verify config effective | Auto-reply triggered when message received |
| 4 | Call POST to disable auto-reply | Auto-reply stops |

---

### TC-CH-015: Group Auto-reply Configuration

**Precondition**: Group created

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/groupchat/{gcid}/auto-reply | Returns current config |
| 2 | Call POST /api/groupchat/{gcid}/auto-reply | Enable auto-reply |
| 3 | Verify config effective | Group message triggers auto-reply |
| 4 | Call DELETE to disable auto-reply | Auto-reply stops |

---

### TC-CH-016: Agent Auto-reply Configuration

**Precondition**: Agent member exists in group

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/groupchat/{gcid}/agents/{agent}/auto-reply | Agent auto-reply enabled |
| 2 | Verify config | Agent participates in group chat |
| 3 | Call DELETE to disable agent auto-reply | Agent stops replying |

---

### TC-CH-017: Semi-automation Mode

**Precondition**: Auto-reply configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Enable semi-automation | Message reply requires human confirmation |
| 2 | Call POST /api/peers/{peer}/half-rewrite | Generate draft reply |
| 3 | Verify draft | Contains suggested reply text |
| 4 | Human modifies and sends | Message sent correctly |

---

### TC-CH-018: Operate Group by GCID

**Precondition**: Group created

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/groupchat/{gcid} | Returns group info |
| 2 | Call POST /api/groupchat/{gcid} | Send message to group |
| 3 | Verify return | Contains gcid, group, name info |

---

### TC-CH-019: File Upload

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/files | File uploaded successfully, returns hash |
| 2 | Verify hash format | SHA256 hash value |
| 3 | Call GET /api/files/{owner}/{hash} | Returns file content |

---

### TC-CH-020: Session File Upload

**Precondition**: Session ID exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call POST /api/files/upload?sessionId=xxx&name=yyy | File uploaded successfully |
| 2 | Verify return | Contains hash, path, name |
| 3 | Call GET /api/files/upload/{sessionId}/{hash} | Returns file content |

---

### TC-CH-021: Message with File

**Precondition**: File uploaded

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Send message with files field | Message stores file reference correctly |
| 2 | Receive message | File info displayed correctly |
| 3 | Download via file URL | File content complete |

---

### TC-CH-022: Message Filter - Credit System

**Precondition**: Filter configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Receive message | Filter evaluation triggered |
| 2 | Message contains sensitive content | Credit score decreased |
| 3 | Credit score too low | Auto-reply paused |
| 4 | Call GET /api/peers/{peer}/auto-reply | Returns current credit score |

---

### TC-CH-023: Message Filter - Keywords

**Precondition**: Keyword filter configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Send message with keyword | Filter triggered |
| 2 | Verify filter result | Message handled according to config |
| 3 | Send normal message | Filter not triggered |

---

### TC-CH-024: Message Filter - Send Filtering

**Precondition**: Send filter configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Auto-reply generates message | Send filter triggered |
| 2 | Filter returns false | Message not sent |
| 3 | Filter returns true | Message sent normally |

---

### TC-CH-025: CLI Command Tests

**Precondition**: Chat App CLI available

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm chat --help` | Display help info |
| 2 | Execute `ztm chat send user1 "hello"` | Message sent successfully |
| 3 | Execute `ztm chat list` | Display chat list |
| 4 | Execute `ztm chat history user1` | Display chat history |

---

### TC-CH-026: GUI Chat Interface

**Precondition**: Chat GUI loaded

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Click user avatar | Enter chat interface |
| 2 | Input message and send | Message displayed in chat window |
| 3 | Receive new message | New message displayed in real-time |
| 4 | Click historical message | Load more history |

---

### TC-CH-027: GUI Group Management

**Precondition**: Chat GUI loaded

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Click create group button | Popup create group dialog |
| 2 | Input group name and members | Group created successfully |
| 3 | Click edit group | Popup edit dialog |
| 4 | Modify group name | Name updated successfully |
| 5 | Add/remove members | Member list updated |
| 6 | Click leave group | Confirm and leave group |
| 7 | Click delete group | Confirm and delete group |

---

### TC-CH-028: GUI Auto-reply Configuration

**Precondition**: Chat GUI loaded, chats exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Open chat settings | Display auto-reply options |
| 2 | Enable auto-reply | Select agent name |
| 3 | Config successful | Display auto-reply enabled |
| 4 | Disable auto-reply | Auto-reply stops |

---

### TC-CH-029: Message Time Filter

**Precondition**: Message history exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/peers/{peer}/messages?since=timestamp | Returns only messages after timestamp |
| 2 | Call GET /api/peers/{peer}/messages?before=timestamp | Returns only messages before timestamp |
| 3 | Use both since and before | Returns messages in time range |

---

### TC-CH-030: Session Messages

**Precondition**: Session exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Send message with sessionId | Message associated with session |
| 2 | Query messages with sessionId | Returns only messages for that session |
| 3 | Session file upload | File associated with session |

---

## Test Environment Requirements

### Hardware Requirements
- At least 2 endpoints (simulating distributed chat environment)
- Optional: Agent environment with openclaw configured

### Software Requirements
- ClawParty runtime
- ZTM network connection
- Chat GUI (optional)

### Test Data
- Test user account
- Test group
- Test message content

---

## Test Execution Guide

### Unit Tests
- Test input/output of each API function
- Verify message format and storage
- Test filter logic

### Integration Tests
- Test CLI command to API call chain
- Test cross-endpoint message delivery
- Test auto-reply trigger

### End-to-End Tests
- Complete chat flow
- Group management flow
- File upload/download flow

### GUI Tests
- Interface interaction tests
- Real-time message updates
- Error handling and user feedback

---

## Known Issues and Notes

1. **Message Format**: Supports plain text and object format with file references
2. **Group ID**: Uses UUID format, generated on creation
3. **GCID**: Group Chat ID, used for quick group lookup
4. **Credit System**: Default credit score 100, filter can decrease
5. **Message Sync**: Based on mesh file system, not real-time push
6. **URL Encoding**: API parameters need URL encoding
7. **Semi-automation**: Requires human confirmation for reply, suitable for quality control scenarios