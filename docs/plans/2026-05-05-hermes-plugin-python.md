# Hermes Python Plugin — Engram Memory Provider

> **Goal:** Create the `~/.hermes/hermes-agent/plugins/memory/engram/` Python plugin so Hermes can use engram as its native memory provider (hooks + tools via HTTP API).

**Architecture:**
- Plugin calls Engram's HTTP API at `localhost:7437`
- Implements `MemoryProvider` ABC from `agent/memory_provider.py`
- 6 MCP-style tools: `engram_search`, `engram_remember`, `engram_context`, `engram_session_start`, `engram_session_end`, `engram_save_prompt`
- Hooks: `on_memory_write` (mirror built-in writes), `on_session_end` (auto-extract)

**Reference plugins:**
- `holographic` — simple, uses `get_config_schema()`, no wizard
- `honcho` — complex, uses custom `post_setup()`

---

### Task 1: Create plugin directory and plugin.yaml

**Objective:** Create the directory and metadata file.

**Files:**
- Create: `~/.hermes/hermes-agent/plugins/memory/engram/plugin.yaml`

```yaml
name: engram
version: 0.1.0
description: "The Crab Engram — persistent memory for AI agents. SQLite + FTS5 + vector search + knowledge graph."
hooks:
  - on_memory_write
  - on_session_end
```

---

### Task 2: Create __init__.py with EngramMemoryProvider class

**Objective:** Implement the MemoryProvider ABC with HTTP calls to `localhost:7437`.

**Files:**
- Create: `~/.hermes/hermes-agent/plugins/memory/engram/__init__.py`
- Requires: `requests` (or use `urllib` from stdlib to avoid pip dep)

**Class structure:**
```python
"""Engram memory provider — The Crab Engram HTTP backend."""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, List
from urllib.request import Request, urlopen
from urllib.error import URLError

from agent.memory_provider import MemoryProvider

logger = logging.getLogger(__name__)

ENGRAM_BASE_URL = "http://localhost:7437"


class EngramMemoryProvider(MemoryProvider):
    """Memory provider backed by the Engram HTTP API."""

    @property
    def name(self) -> str:
        return "engram"

    def is_available(self) -> bool:
        """Check if Engram server is reachable."""
        try:
            req = Request(f"{ENGRAM_BASE_URL}/health")
            with urlopen(req, timeout=2) as resp:
                return resp.status == 200
        except (URLError, OSError):
            return False

    def initialize(self, session_id: str, **kwargs) -> None:
        """Start a session via API."""
        self._session_id = session_id
        self._project = kwargs.get("project", "default")
        try:
            self._api_post("/session/start", {"project": self._project})
        except Exception as e:
            logger.warning("Engram: failed to start session: %s", e)

    def get_tool_schemas(self) -> List[Dict[str, Any]]:
        return [SEARCH_SCHEMA, REMEMBER_SCHEMA, CONTEXT_SCHEMA,
                SESSION_START_SCHEMA, SESSION_END_SCHEMA, SAVE_PROMPT_SCHEMA]

    def handle_tool_call(self, tool_name: str, args: Dict[str, Any], **kwargs) -> str:
        handlers = {
            "engram_search": self._handle_search,
            "engram_remember": self._handle_remember,
            "engram_context": self._handle_context,
            "engram_session_start": self._handle_session_start,
            "engram_session_end": self._handle_session_end,
            "engram_save_prompt": self._handle_save_prompt,
        }
        handler = handlers.get(tool_name)
        if not handler:
            return json.dumps({"error": f"Unknown tool: {tool_name}"})
        return json.dumps(handler(args))

    def on_memory_write(self, action: str, target: str, content: str, metadata=None) -> None:
        """Mirror built-in memory writes to Engram."""
        if action == "add":
            try:
                self._api_post("/remember", {
                    "content": f"{target}: {content}",
                    "memory_type": "preference" if target == "user" else "fact",
                    "session_id": self._session_id,
                })
            except Exception as e:
                logger.debug("Engram: memory write mirror failed: %s", e)

    def on_session_end(self, messages: List[Dict[str, Any]]) -> None:
        """End session + auto-extract passive learnings."""
        try:
            # Extract assistant messages
            outputs = [m.get("content", "") for m in messages if m.get("role") == "assistant"]
            if outputs:
                combined = "\n".join(outputs[-5:])
                self._api_post("/capture/passive", {
                    "output": combined,
                    "session_id": self._session_id,
                })
            self._api_post(f"/session/{self._session_id}/end", {
                "summary": f"Session with {len(messages)} turns"
            })
        except Exception as e:
            logger.debug("Engram: session end failed: %s", e)

    # --- Internal API helpers ---

    def _api_post(self, path: str, data: dict) -> dict:
        body = json.dumps(data).encode()
        req = Request(f"{ENGRAM_BASE_URL}{path}", data=body,
                       headers={"Content-Type": "application/json"})
        with urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode())

    def _api_get(self, path: str) -> dict:
        req = Request(f"{ENGRAM_BASE_URL}{path}")
        with urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode())

    # --- Tool handlers ---

    def _handle_search(self, args: dict) -> dict:
        query = args.get("query", "")
        limit = args.get("limit", 10)
        result = self._api_get(f"/observations?query={query}&limit={limit}")
        return {"results": result}

    def _handle_remember(self, args: dict) -> dict:
        result = self._api_post("/remember", {
            "content": args["content"],
            "memory_type": args.get("memory_type", "fact"),
            "session_id": self._session_id,
        })
        return {"id": result.get("id")}

    def _handle_context(self, args: dict) -> dict:
        limit = args.get("limit", 10)
        result = self._api_get(f"/observations/context?limit={limit}")
        return {"context": result}

    def _handle_session_start(self, args: dict) -> dict:
        result = self._api_post("/session/start", {})
        return {"session_id": result.get("id")}

    def _handle_session_end(self, args: dict) -> dict:
        summary = args.get("summary", "")
        self._api_post(f"/session/{self._session_id}/end", {"summary": summary})
        return {"status": "ended"}

    def _handle_save_prompt(self, args: dict) -> dict:
        self._api_post("/capture/prompt", {
            "content": args["content"],
            "session_id": self._session_id,
        })
        return {"status": "saved"}
```

---

### Task 3: Create tool schemas

**Objective:** Define the 6 tool schemas following OpenAI function-calling format.

```python
SEARCH_SCHEMA = {
    "name": "engram_search",
    "description": "Search Engram memory by keyword. Returns ranked observations.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search keywords"},
            "limit": {"type": "integer", "description": "Max results (default 10)"},
        },
        "required": ["query"],
    },
}

REMEMBER_SCHEMA = {
    "name": "engram_remember",
    "description": "Store a fact or observation in Engram.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Content to remember"},
            "memory_type": {
                "type": "string",
                "enum": ["fact", "preference", "decision", "learning"],
                "description": "Type of memory",
            },
        },
        "required": ["content"],
    },
}

CONTEXT_SCHEMA = {
    "name": "engram_context",
    "description": "Get recent session context from Engram.",
    "parameters": {
        "type": "object",
        "properties": {
            "limit": {"type": "integer", "description": "Max observations (default 10)"},
        },
        "required": [],
    },
}

SESSION_START_SCHEMA = {
    "name": "engram_session_start",
    "description": "Start a new Engram session.",
    "parameters": {"type": "object", "properties": {}, "required": []},
}

SESSION_END_SCHEMA = {
    "name": "engram_session_end",
    "description": "End the current Engram session.",
    "parameters": {
        "type": "object",
        "properties": {
            "summary": {"type": "string", "description": "Session summary"},
        },
        "required": [],
    },
}

SAVE_PROMPT_SCHEMA = {
    "name": "engram_save_prompt",
    "description": "Save a user prompt to Engram.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Prompt content"},
        },
        "required": ["content"],
    },
}
```

---

### Task 4: Configure Hermes to use the plugin

**Objective:** Set `memory.provider: engram` in `~/.hermes/config.yaml` and verify the plugin loads.

**Files:**
- Modify: `~/.hermes/config.yaml`

Set:
```yaml
memory:
  provider: engram
```

**Verification:**
1. Restart Hermes session
2. Check logs: `grep -i engram ~/.hermes/logs/agent.log`
3. Run `hermes tools | grep engram` — should show 6 tools
4. Call `engram_search(query="test")` — should return results from Engram DB

**Potential issues:**
- The `/remember` and `/session/start` endpoints must exist in the Engram API. Verify with `curl localhost:7437/health` first.
- The plugin needs the Engram server running (`systemctl --user status engram`).
