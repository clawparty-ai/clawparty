# Cloud App Test Plan

## Overview

Cloud App is ClawParty distributed file cloud storage application, supporting cross-endpoint file sync, sharing and management. This document records Cloud App test plans and test cases.

## Test Scope

### 1. Basic Function Tests
- Endpoint management
- Configuration management
- File list
- File status query

### 2. File Upload Tests
- Single file upload
- Large file chunked upload
- Duplicate file upload

### 3. File Download Tests
- Single file download
- Chunked download
- Concurrent download
- Download resume
- Download cancel

### 4. File Management Tests
- File deletion (unpublish)
- File erasure (erase)
- Directory operations

### 5. Access Control Tests
- ACL settings
- Permission verification
- Cross-user access

### 6. Auto-sync Tests
- Auto-download
- Auto-upload
- Mirror configuration

### 7. CLI Command Tests
- config command
- ls command
- share command
- mirror command
- upload/download command
- unpublish/erase command

---

## Test Cases

### TC-CL-001: Endpoint Configuration Management

**Precondition**: Cloud App is running

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud config --local-dir ~/testCloud` | Config updated successfully |
| 2 | Execute `ztm cloud config` | Display localDir as ~/testCloud |
| 3 | Call GET /api/endpoints/{ep}/config | Returns current config |

---

### TC-CL-002: File List Query

**Precondition**: Test files exist

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud ls /` | Display user list |
| 2 | Execute `ztm cloud ls /users/{username}` | Display user's file list |
| 3 | Execute `ztm cloud ls /users/{username} --hash` | Display file info and hash |
| 4 | Call GET /api/files/ | Returns root directory info |
| 5 | Call GET /api/files/users/{username} | Returns user file list |

---

### TC-CL-003: File Upload

**Precondition**: Local file exists for upload

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud upload /users/{username}/test.txt` | File uploaded successfully |
| 2 | Call POST /api/uploads {path: "path"} | Returns 201 |
| 3 | Execute `ztm cloud ls` after upload | File status becomes synced |
| 4 | Upload same file again | Returns success (idempotent) |

---

### TC-CL-004: File Download

**Precondition**: Downloadable file exists on remote

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud download /users/{username}/test.txt` | Download task started |
| 2 | Execute `ztm cloud download --list` | Display download progress |
| 3 | Call GET /api/downloads | Returns download list |
| 4 | Check local file after download | File complete, hash matches |

---

### TC-CL-005: Chunked Download and Resume

**Precondition**: Large file (>1MB) has chunks

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Start downloading large file | Chunked download started |
| 2 | Interrupt download (simulate network failure) | Download paused |
| 3 | Restart Cloud App | Auto-resume download |
| 4 | Execute `ztm cloud download --list` | Display download progress continues |

---

### TC-CL-006: Download Cancel

**Precondition**: Download in progress exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Start downloading large file | Download task started |
| 2 | Execute `ztm cloud download --cancel /path/to/file` | Download task cancelled |
| 3 | Execute `ztm cloud download --list` | Download no longer displayed |
| 4 | Call DELETE /api/downloads/{path} | Returns 204 |

---

### TC-CL-007: File Deletion (Unpublish)

**Precondition**: File uploaded and synced

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud unpublish /users/{username}/test.txt` | File deleted from cloud |
| 2 | Execute `ztm cloud ls` again | File no longer displayed |
| 3 | Local file retained | Local file unaffected |
| 4 | Call DELETE /api/files/{path} | Returns 204 |

---

### TC-CL-008: File Erasure (Erase)

**Precondition**: File has been uploaded

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud erase /users/{username}/test.txt` | Local file deleted |
| 2 | Call DELETE /api/endpoints/{ep}/files/{path} | Returns 204 |
| 3 | Cloud metadata retained | Other endpoints can still download |

---

### TC-CL-009: ACL Permission Settings

**Precondition**: File has been uploaded

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud share /path --set-all readonly` | All users can read |
| 2 | Execute `ztm cloud share /path --set-block user1` | user1 blocked |
| 3 | Execute `ztm cloud share /path --set-readonly user2` | user2 read-only access |
| 4 | Execute `ztm cloud share /path` | Display current ACL config |
| 5 | Call POST /api/acl/{path} | ACL set successfully |

---

### TC-CL-010: ACL Permission Verification

**Precondition**: ACL has been set

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Blocked user tries to download file | Returns 403 or access denied |
| 2 | Read-only user tries to download file | Download successful |
| 3 | Read-only user tries to upload file | Upload rejected |
| 4 | Call GET /api/acl/{path} | Returns ACL config |

---

### TC-CL-011: Auto-download Configuration

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud mirror /users/remoteUser --download on` | Configure auto-download |
| 2 | Remote user uploads new file | Auto-download started |
| 3 | Execute `ztm cloud mirror /users/remoteUser` | Display download: on |
| 4 | Execute `ztm cloud mirror /users/remoteUser --download off` | Disable auto-download |

---

### TC-CL-012: Auto-upload Configuration

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud mirror /users/{username} --upload on` | Configure auto-upload |
| 2 | Local file modified | Auto-upload started |
| 3 | Execute `ztm cloud mirror /users/{username}` | Display upload: on |
| 4 | Execute `ztm cloud mirror /users/{username} --upload off` | Disable auto-upload |

---

### TC-CL-013: Mirror Configuration View

**Precondition**: Mirror configured

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/mirrors | Returns all mirror configs |
| 2 | Call GET /api/mirrors/{path} | Returns mirror config for specified path |
| 3 | Call GET /api/endpoints/{ep}/mirrors | Returns mirror config for specified endpoint |

---

### TC-CL-014: File Streaming

**Precondition**: File has been uploaded

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/file-data/{path} | Returns file stream |
| 2 | Verify Content-Type | Correctly set based on extension |
| 3 | Verify file integrity | Downloaded file hash matches |

---

### TC-CL-015: Chunk Serving

**Precondition**: File has chunks

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/chunks/users/{username}/{file}?chunk=0 | Returns first chunk data |
| 2 | Call GET /api/chunks/users/{username}/{file}?chunk=1 | Returns second chunk data |
| 3 | Verify chunk hash | Chunk hash matches metadata |

---

### TC-CL-016: Concurrent Download

**Precondition**: Large file needs chunked download

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Start downloading large file | Start 8 concurrent download channels |
| 2 | Monitor download progress | Multiple chunks download simultaneously |
| 3 | Download complete | All chunks merged, file complete |

---

### TC-CL-017: Download to Specified File

**Precondition**: File exists on remote

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud download /remote/path -o /local/output.txt` | File saved to specified location |
| 2 | Verify output file | File content and hash correct |

---

### TC-CL-018: Directory Sync

**Precondition**: Directory exists

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Execute `ztm cloud mirror /dir --download on --upload on` | Bidirectional sync configured |
| 2 | Local file modified | Auto-upload to cloud |
| 3 | Remote file modified | Auto-download to local |
| 4 | Verify file status | Status is synced |

---

### TC-CL-019: Cross-endpoint Config Sync

**Precondition**: Multi-endpoint environment

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Call GET /api/endpoints/{ep}/config | Returns endpoint config |
| 2 | Call POST /api/endpoints/{ep}/config | Config updated successfully |
| 3 | Verify config applied | New config takes effect |

---

### TC-CL-020: Error Handling

**Precondition**: None

| Step | Action | Expected Result |
|------|--------|-----------------|
| 1 | Access non-existent file | Returns 404 |
| 2 | Access file without permission | Returns 403 |
| 3 | Invalid API call | Returns appropriate error code |
| 4 | Source endpoint goes offline during download | Auto-try other sources |

---

## Test Environment Requirements

### Hardware Requirements
- At least 2 endpoints (simulating distributed environment)
- Enough disk space for file tests

### Software Requirements
- ClawParty runtime
- ZTM network connection

### Test Data
- Small files (<1MB)
- Large files (>10MB, for chunked tests)
- Multiple files (for directory tests)

---

## Test Execution Guide

### Unit Tests
- Test input/output of each API function
- Verify file status calculation logic
- Test hash calculation

### Integration Tests
- Test CLI command to API call chain
- Test cross-endpoint file sync
- Test ACL permission verification

### End-to-End Tests
- Complete file upload/download flow
- Auto-sync scenarios
- Error recovery scenarios

---

## Known Issues and Notes

1. **Chunk size**: Default is 1MB, modification requires recompilation
2. **Concurrency**: Default 8 concurrent download channels
3. **Path handling**: Uses `os.path.normalize` to unify path format
4. **ACL inheritance**: Directory ACL does not automatically inherit to sub-files