# Radar Methodology

## Overview

Radar is a structured intelligence-gathering system for zAgents. It helps an agent systematically discover subjects of interest (targets) through configurable detection configurations (probes), and maintain up-to-date information about those targets through periodic scanning.

This document defines the radar methodology that an LLM/agent follows when a user says: "Help me set up radar, I care about X."

## Core Concepts

### Probe
A **probe** is a detection configuration. It describes:

- **What to look for**: Characteristics, patterns, or descriptions of subjects of interest
- **How to detect**: The detection method or approach (keyword matching, pattern recognition, data analysis, relationship mapping, etc.)
- **Channel**: The source(s) to monitor, such as websites, URLs, document repositories, APIs, RSS feeds, social media, databases, or custom data sources

Probes answer: *"What am I looking for, how do I find it, and where do I look?"*

### Target
A **target** is a subject of interest that has been identified (either directly by the user or discovered through a probe). Each target has:

- **Description**: What/who the target is
- **Spec**: Technical specifications, attributes, or structured data about the target
- **Channel(s)**: Sources through which the target is periodically scanned for updates
- **Status**: Active, monitoring, paused, archived

Targets answer: *"Who/what am I tracking, what do I know about it, and where do I watch for changes?"*

## Directory Structure

```
workspace/radar/
├── radar.md           # This methodology document (for agent reference)
├── probes.md          # Master list of all probes (YAML frontmatter + Markdown)
├── targets.md         # Master list of all targets (YAML frontmatter + Markdown)
└── logs/
    ├── probe-<YYYYMMDD-HHMMSS>.md   # Probe execution log
    └── scan-<YYYYMMDD-HHMMSS>.md    # Target scan execution log
```

## File Format

### probes.md

YAML frontmatter with a list of probes. Each probe has:

```yaml
probes:
  - id: probe-001
    name: "AI Startup Tracker"
    description: "Monitor new AI startup announcements"
    channel:
      type: website
      location: "https://news.ycombinator.com/"
      selector: "new AI startup" 
    method: keyword_match
    schedule: daily
    status: active
    created_at: "2026-05-19T10:00:00+08:00"
    last_run: "2026-05-19T10:30:00+08:00"
```

### targets.md

YAML frontmatter with a list of targets. Each target has:

```yaml
targets:
  - id: target-001
    name: "ExampleCorp AI"
    description: "ExampleCorp's new AI division"
    spec:
      founded: "2026-03"
      funding: "$50M"
      product: "ExampleGPT"
      headquarters: "San Francisco, CA"
    channels:
      - type: website
        location: "https://examplecorp.com/ai"
      - type: rss
        location: "https://examplecorp.com/ai/blog/feed.xml"
    source_probe: probe-001
    status: monitoring
    created_at: "2026-05-19T10:30:00+08:00"
    last_scan: "2026-05-19T11:00:00+08:00"
```

### Log Files

Log files are plain Markdown with a timestamp in the filename:

```markdown
# Probe Log: probe-001 (AI Startup Tracker)

**Run**: 2026-05-19 10:30:00
**Channel**: https://news.ycombinator.com/
**Status**: completed

## Results

- Found 3 potential matches
- New targets identified: 1
- [Target-002: NovaAI](/radar/targets/novaai)

## Notes

Scanned front page and "new" section. One interesting post about NovaAI.
```

## Workflow

### Phase 1: Initialization

When a user says "Help me set up radar, I care about X":

1. **Understand the user's focus**: Clarify what the user wants to track. Is it a specific target ("I care about Company ABC") or a category/probe ("I care about new AI startups")?

2. **Initialize radar directory**: Create the `radar/` directory structure if it does not exist.

3. **Generate probes.md**: Based on the user's description, create an initial set of probes. Each probe should have a clear channel, method, and schedule.

4. **Generate targets.md**: If the user described specific targets, create entries for them directly. If they described characteristics (probes), the targets list will be populated by probe execution.

### Phase 2: Probe Execution

Periodic probe execution (typically daily or on a custom schedule):

1. Read the active probes from `probes.md`
2. For each due probe, execute the detection method on its channel
3. Log the execution in `logs/probe-<timestamp>.md`
4. Update `probes.md` with the `last_run` timestamp
5. Any new discoveries are added to `targets.md` as new entries

### Phase 3: Target Scanning

Periodic target scanning (typically on a custom schedule for each target):

1. Read active targets from `targets.md`
2. For each target, scan its channels for updates
3. Log the scan in `logs/scan-<timestamp>.md`
4. Update `targets.md` — refresh `spec` fields, update `last_scan`, adjust `status` if needed
5. If scanning reveals new related targets, add them to `targets.md` and consider creating new probes

### Phase 4: Maintenance

- **Probe refinement**: If probes consistently return no results, suggest adjusting the channel or method
- **Target lifecycle**: Mark targets as `paused` or `archived` when no longer relevant
- **Log rotation**: Old logs may be summarized into periodic reports and pruned

## Channel Types

| Type | Description | Example |
|------|-------------|---------|
| `website` | Web page or site to scrape/monitor | `https://example.com/news` |
| `rss` | RSS/Atom feed | `https://example.com/feed.xml` |
| `api` | REST API endpoint | `https://api.example.com/v1/items` |
| `document` | Local or remote document store | `workspace/radar/docs/` |
| `social` | Social media feed | Twitter/X hashtag, Reddit subreddit |
| `custom` | User-defined tool/script | Python scraper, shell script |

## Tool Integration

For probe execution and target scanning, the agent should:

1. **Use built-in capabilities**: HTTP requests, file I/O, data parsing
2. **Prompt the user for tools**: If a channel requires specialized tooling (e.g., a custom web scraper, API client, or data parser), describe what tool is needed and ask the user to create it (e.g., a Python script)
3. **Log tool usage**: Record which tools were used in the corresponding log files

## Scheduling

Probes and scans run on schedules defined in their entries. The scheduling system:

- Checks `probes.md` and `targets.md` periodically (every hour by default)
- Runs any probe or scan whose `schedule` interval has elapsed since `last_run`
- Can be integrated with the agent's cron/scheduled task system

## Relationship with Wiki

Radar and Wiki are complementary:

- **Wiki**: Knowledge base — structured information that the agent maintains
- **Radar**: Intelligence gathering — discovering and tracking subjects of interest

Probe discoveries can feed into Wiki pages, and Wiki content can inform probe refinement.
