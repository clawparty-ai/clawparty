# ClawParty Test Scripts

## Directory Structure

```
tests/
├── prepare.sh           # Initialize test environment
├── cleanup.sh           # Clean up test environment
├── run-tests.sh         # Run all tests
├── api/                 # API tests
│   ├── run-tests.sh
│   ├── hub/             # Hub API tests
│   │   ├── test-hub-status.sh
│   │   ├── test-hub-log.sh
│   │   ├── test-hub-zones.sh
│   │   ├── test-hub-endpoints.sh
│   │   └── test-hub-users.sh
│   └── agent/           # Agent API tests
│       ├── run-tests.sh
│       ├── test-agent-version.sh
│       ├── test-agent-identity.sh
│       ├── test-agent-meshes-list.sh
│       └── apps/        # App API tests (to be implemented)
└── cli/                 # CLI tests
    ├── run-tests.sh
    ├── test-help.sh
    ├── test-version.sh
    ├── test-version-json.sh
    ├── test-identity-agent1.sh
    └── test-identity-agent2.sh
```

## Quick Start

### 1. Initialize Test Environment

```bash
cd tests
./prepare.sh
```

This will start:
- Hub: http://localhost:18888
- Agent1: http://localhost:7778
- Agent2: http://localhost:7779

### 2. Run Tests

```bash
# Run all tests
./run-tests.sh

# Run individually
./api/run-tests.sh      # API tests
./cli/run-tests.sh      # CLI tests
```

### 3. Clean Up

```bash
./cleanup.sh
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| HUB_URL | http://localhost:18888 | Hub service address |
| AGENT_URL | http://localhost:7778 | Agent1 service address |
| AGENT2_URL | http://localhost:7779 | Agent2 service address |
| API_TOKEN | enjoyParty | API authentication token |
| ZTM_CONFIG | 127.0.0.1:7778 | CLI connected Agent address |
| ZTM_API_TOKEN | enjoy-party | CLI authentication token |

## Test Coverage

### Hub API (18 tests)
- /api/status
- /api/log
- /api/zones
- /api/endpoints
- /api/users
- etc...

### Agent API (43 tests)
- /api/version
- /api/identity
- /api/meshes
- etc...

### CLI Commands (74 tests)
- help, version, config
- identity, get, describe
- mesh/endpoint/app operations
- chat, tunnel, cloud, proxy, payment app commands

## Test Status

### Implemented
- Environment initialization script
- Hub API basic tests
- Agent API basic tests
- CLI basic tests

### To Be Implemented
- App API tests (requires mesh and app running)
- Mesh operation related tests
- File transfer tests
- Payment tests
- More edge case tests