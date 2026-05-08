# Pipy Upgrade Report

## Overview

The `pipy/` directory was replaced with a new version of the Pipy runtime. This report documents the differences between the previous version (tracked in Git) and the newly copied version.

## Statistics

| Metric | Count |
|--------|-------|
| Modified files | 2 |
| Added files | 11 |
| Deleted files | 0 |

---

## Modified Files

### 1. `pipy/CMakeLists.txt`

**What changed:** Removed the automatic dependency collection for custom codebases in the CMake build.

**Details:**

```diff
-# Collect all source files from custom codebases as dependencies
-set(CODEBASE_DEPS pack ${CMAKE_SOURCE_DIR}/src/scripts ${CMAKE_SOURCE_DIR}/samples)
-if(PIPY_CUSTOM_CODEBASES)
-  string(REPLACE "," ";" CUSTOM_CODEBASE_LIST "${PIPY_CUSTOM_CODEBASES}")
-  foreach(CB ${CUSTOM_CODEBASE_LIST})
-    string(REPLACE ":" ";" CB_PARTS "${CB}")
-    list(GET CB_PARTS 1 CB_PATH)
-    file(GLOB_RECURSE CB_FILES "${CMAKE_SOURCE_DIR}/${CB_PATH}/*.js" "${CMAKE_SOURCE_DIR}/${CB_PATH}/*.json" "${CMAKE_SOURCE_DIR}/${CB_PATH}/*.html" "${CMAKE_SOURCE_DIR}/${CB_PATH}/*.css" "${CMAKE_SOURCE_DIR}/${CB_PATH}/*.ico")
-    list(APPEND CODEBASE_DEPS ${CB_FILES})
-  endforeach()
-endif()

 add_custom_command(
   OUTPUT ${CMAKE_BINARY_DIR}/deps/codebases.br.h
   COMMAND ${CMAKE_BINARY_DIR}/${EXE_PACK}
   ARGS ${CMAKE_BINARY_DIR}/deps/codebases.br.h ${CODEBASES} /samples/nmi/,/samples/bpf/
   WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
-  DEPENDS ${CODEBASE_DEPS}
+  DEPENDS pack ${CMAKE_SOURCE_DIR}/src/scripts ${CMAKE_SOURCE_DIR}/samples
 )
```

**Impact:** Simplifies the build by removing `GLOB_RECURSE`-based dependency tracking for custom codebases. The `pack` tool now only tracks core scripts and samples, not arbitrary files in custom codebase directories. This makes incremental builds more predictable and avoids CMake reconfiguration when non-matching files change in custom directories.

---

### 2. `pipy/src/filters/tls.cpp`

**What changed:** Added backward-compatible TLS cipher configuration for TLS 1.2 and older versions.

**Details:**

```diff
 void TLSContext::set_ciphers(const std::string &ciphers) {
   SSL_CTX_set_ciphersuites(m_ctx, ciphers.c_str());
+  SSL_CTX_set_cipher_list(m_ctx, ciphers.c_str()); // for TLS1.2 and below
 }
```

**Impact:** Previously, `SSL_CTX_set_ciphersuites()` was called, which only affects TLS 1.3 cipher suites. The new code also calls `SSL_CTX_set_cipher_list()`, which configures cipher suites for TLS 1.2 and earlier. This ensures that custom cipher configurations work across all TLS protocol versions, not just 1.3.

**Compatibility:** This is a bugfix for environments that restrict TLS versions below 1.3.

---

## Added Files

### Documentation (9 files)

New API reference documentation for the `Data` class:

| File | Size | Description |
|------|------|-------------|
| `pipy/docs/reference/api/Data/from.mdx` | 33 lines | `Data.from()` method reference |
| `pipy/docs/reference/api/Data/new.mdx` | 36 lines | `Data()` constructor reference |
| `pipy/docs/reference/api/Data/push.mdx` | 24 lines | `Data.push()` method reference |
| `pipy/docs/reference/api/Data/shift.mdx` | 23 lines | `Data.shift()` method reference |
| `pipy/docs/reference/api/Data/shiftTo.mdx` | 26 lines | `Data.shiftTo()` method reference |
| `pipy/docs/reference/api/Data/shiftWhile.mdx` | 26 lines | `Data.shiftWhile()` method reference |
| `pipy/docs/reference/api/Data/size.mdx` | 18 lines | `Data.size` property reference |
| `pipy/docs/reference/api/Data/toArray.mdx` | 23 lines | `Data.toArray()` method reference |
| `pipy/docs/reference/api/Data/toString.mdx` | 31 lines | `Data.toString()` method reference |

**Total:** ~260 lines of new API documentation.

### OpenSSL Samples (2 files)

**Note:** These are PEM key files included as examples/demos in the bundled OpenSSL 3.5.2 dependency. They are purely for OpenSSL's own documentation and demo purposes, not used by Pipy at runtime.

| File | Size | Description |
|------|------|-------------|
| `pipy/deps/openssl-3.5.2/apps/rsa8192.pem` | Small | RSA 8192-bit demo key (OpenSSL app sample) |
| `pipy/deps/openssl-3.5.2/demos/smime/cakey.pem` | Small | CA private key demo (OpenSSL SMIME demo) |

**Impact:** No functional impact. These files are part of the OpenSSL source tree that Pipy bundles.

---

## Deleted Files

None.

---

## Summary of Changes by Category

| Category | Files | Notes |
|----------|-------|-------|
| **Build System** | 1 | CMakeLists.txt simplified (removed custom codebase deps) |
| **TLS/Security** | 1 | Fixed cipher configuration for TLS 1.2 and below |
| **Documentation** | 9 | New `Data` API reference pages |
| **Dependencies** | 2 | OpenSSL demo/sample PEM files |

---

## Recommended `.gitignore` Updates

The new `pipy/` directory contains build and cache artifacts that should be ignored. The following rules should be added to the root `.gitignore` (or merge `pipy/.gitignore` contents):

```gitignore
# Pipy build outputs
pipy/bin/
pipy/build/
pipy/public/

# Pipy runtime data
pipy/logs/
pipy/pids/

# Gatsby cache (Pipy docs)
pipy/.cache/

# Node
pipy/node_modules/
pipy/.npm
pipy/.eslintcache

# Package output
pipy/*.tgz

# Other
pipy/.grunt
pipy/.lock-wscript
pipy/.node_repl_history
pipy/typings/
pipy/jspm_packages/
pipy/bower_components/
pipy/lib-cov
pipy/coverage
pipy/.nyc_output
```

---

## Notes

1. The `pipy/` directory is not a Git submodule; files are tracked directly in the main repository.
2. The OpenSSL version bundled remains **3.5.2** (unchanged).
3. All tracked files in `pipy/` previously numbered **14,093**; after the replacement there are **14,104** files (net +11).
4. No source code files were deleted or moved; the only source change is the TLS cipher fix in `tls.cpp`.

---

## PipyJS Engine Capability Audit

After the pipy replacement, the PipyJS engine (`pipy/src/pjs/`) was audited by reading the C++ source directly (`builtin.cpp`, `stmt.cpp`, `stmt.hpp`, `parser.cpp`, `types.cpp`, `types.hpp`, `expr.cpp`). The audit revealed that five limitations previously documented in `AGENTS.md` are **no longer accurate** in the current runtime, while four restrictions remain valid.

### Previously Documented but Now Supported

| Feature | Previous `AGENTS.md` Claim | Engine Evidence | Status |
|---------|---------------------------|-----------------|--------|
| **RegExp** | "Does not support RegExp APIs" | `RegExp` class fully implemented with `exec()`, `test()`, constructor, and flags handling (`types.cpp:3526+`) | ✅ Supported |
| **Arrow functions** | "Does not support `=>`" | Parser tokenizes `=>` (`parser.cpp:302`); test evals include `((x,y)=>x+y)(1,2)` (`main.cpp:163`) | ✅ Supported |
| **`continue`/`break`** | "Does not support in loops" | `Stmt::Result` enum defines `BREAK` and `CONTINUE` (`stmt.hpp:48`); `For::execute()` handles both (`stmt.cpp:520+`) | ✅ Supported |
| **`while` loops** | "Does not support `while`" | Parser tokenizes `while` (`parser.cpp:315`); parser uses `while (!eof())` patterns throughout | ✅ Supported |
| **Function hoisting** | "Functions must be defined before called" | Need verification, but engine structure supports it | Likely supported |

### Still Accurate Restrictions

| Feature | Evidence | Recommendation |
|---------|----------|----------------|
| **Array `.map()` / `.filter()` / `.reduce()` / `.forEach()`** | `Array` class present but no `map/filter/reduce/forEach` methods defined in `builtin.cpp` | Use `for` loops |
| **`Number.isNaN()`** | Only global `isNaN()` found; no `Number.isNaN` binding in `builtin.cpp` | Use `isNaN(value)` or try-catch with `Number()` |
| **`Date.toLocaleTimeString()` / `toLocaleDateString()`** | `Date` class lacks locale methods | Format manually with `getHours()`, `getMinutes()`, etc. |
| **`os.read()` returns Data** | Confirmed by `types.hpp` Data class | Always call `.toString()` before string operations |

### Impact on Existing Code

The existing Pipy-side codebase (`cli/`, `agent/`, `hub/`, `ca/`) still follows the older patterns (no RegExp, no arrow functions, no `continue`/`break`, no `while`). When editing those files, **keep file-local style consistent** with surrounding code rather than modernizing for modernization's sake, even though the runtime now supports the newer syntax.
