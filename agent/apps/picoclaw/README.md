# Picoclaw ZTM App

This ZTM app integrates Picoclaw with ClawParty, treating Picoclaw as a single agent participant.

## Overview

Picoclaw is a **single agent system** - when it starts, it IS the agent (unlike OpenClaw which manages multiple agents). This app provides a simple bridge between ClawParty chat and Picoclaw.

## API Endpoints

### Health Check
```
GET /api/picoclaw/health

Response 200:
{
  "status": "online",
  "agent": "picoclaw",
  "output": "picoclaw Status\nVersion: ..."
}

Response 503:
{
  "status": "offline",
  "agent": "picoclaw",
  "output": "command not found"
}
```

### Chat with Picoclaw
```
POST /api/picoclaw/chat
Content-Type: application/json

{
  "message": "Hello Picoclaw",
  "session_id": "optional-session-id"
}

Response 200:
{
  "agent": "picoclaw",
  "response": "Hello! How can I help you?",
  "session_id": "clawparty:1234567890"
}

Response 500:
{
  "error": "Picoclaw command failed",
  "output": "error details"
}
```

## Requirements

- Picoclaw CLI must be installed and available in PATH
- Run `which picoclaw` to verify installation

## Usage with ClawParty

```bash
# 1. Start Picoclaw gateway
picoclaw gateway &

# 2. Start ClawParty agent
./bin/ztm run agent --listen :6789

# 3. Test integration
curl http://localhost:6789/api/picoclaw/health
```

## Testing

```bash
# Health check
curl http://localhost:6789/api/picoclaw/health

# Send message
curl -X POST http://localhost:6789/api/picoclaw/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "What is 2+2?"}'
```