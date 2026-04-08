# MCP Server Specification

## Purpose

Servidor MCP con ~35 tools y 3 resources, organizados en profiles (agent/admin/all), compatible con rmcp 1.3+.

## Requirements

### Requirement: MCP Server Initialization

The system MUST initialize an MCP server with rmcp over stdio transport.

#### Scenario: Server starts with stdio transport

- GIVEN SqliteStore and config
- WHEN `the-crab-engram mcp` is called
- THEN EngramServer starts listening on stdio

#### Scenario: Tool profiles filter available tools

- GIVEN profile=agent (default)
- WHEN tools are listed
- THEN 30 tools are available (excludes admin-only: mem_delete, mem_stats, mem_timeline, mem_merge_projects, mem_consolidate)

### Requirement: F1 Parity Tools (15 tools)

The system MUST implement 15 MCP tools matching Engram Go.

#### Scenario: mem_save stores observation

- GIVEN valid title, content, type, scope, session_id, project
- WHEN mem_save is called
- THEN observation is stored, embedding generated, observation_id returned

#### Scenario: mem_search returns ranked results

- GIVEN query string and optional filters
- WHEN mem_search is called
- THEN ranked results with relevance scores are returned

#### Scenario: mem_context returns session context

- GIVEN project name
- WHEN mem_context is called
- THEN last N observations from most recent session are returned

#### Scenario: mem_session_start creates session

- GIVEN project name
- WHEN mem_session_start is called
- THEN new session with UUID is created, session_id returned

#### Scenario: mem_session_end closes session

- GIVEN session_id and optional summary
- WHEN mem_session_end is called
- THEN session is marked ended with timestamp

#### Scenario: mem_get_observation increments access

- GIVEN observation_id
- WHEN mem_get_observation is called
- THEN observation returned, access_count incremented

#### Scenario: mem_update modifies observation

- GIVEN observation_id and fields to update
- WHEN mem_update is called
- THEN observation is updated, updated_at set

#### Scenario: mem_delete removes observation

- GIVEN observation_id and mode (soft/hard)
- WHEN mem_delete is called (admin only)
- THEN observation is deleted accordingly

#### Scenario: mem_suggest_topic_key generates key

- GIVEN type and title
- WHEN mem_suggest_topic_key is called
- THEN suggested topic_key in format "type/slug" is returned

#### Scenario: mem_timeline returns chronological context

- GIVEN observation_id and window
- WHEN mem_timeline is called (admin only)
- THEN observations before and after in time are returned

#### Scenario: mem_capture_passive extracts learnings

- GIVEN agent output text
- WHEN mem_capture_passive is called
- THEN learnings are extracted and stored as observations with inferred provenance and salience

#### Scenario: mem_save_prompt stores user prompt

- GIVEN prompt text and session_id
- WHEN mem_save_prompt is called
- THEN prompt is stored for future context

#### Scenario: mem_stats returns statistics

- GIVEN project
- WHEN mem_stats is called (admin only)
- THEN total observations, by type, by scope, sessions count are returned

#### Scenario: mem_merge_projects combines projects

- GIVEN source and target project names
- WHEN mem_merge_projects is called (admin only)
- THEN all observations from source are moved to target

### Requirement: F2-2.5 New Tools (17 tools)

The system MUST implement additional tools for search, graph, and auto-learning.

#### Scenario: mem_relate creates edge

- GIVEN source_id, target_id, relation_type
- WHEN mem_relate is called
- THEN edge is created in graph (with temporal auto-closure)

#### Scenario: mem_graph returns graph JSON

- GIVEN observation_id
- WHEN mem_graph is called
- THEN JSON graph of related observations is returned (only active edges by default)

#### Scenario: mem_graph_timeline shows evolution

- GIVEN observation_id
- WHEN mem_graph_timeline is called
- THEN historical edges are shown with validity windows

#### Scenario: mem_pin toggles pinned state

- GIVEN observation_id
- WHEN mem_pin is called
- THEN pinned state is toggled, affects scoring

#### Scenario: mem_reembed regenerates embeddings

- GIVEN project or all observations
- WHEN mem_reembed is called
- THEN embeddings are regenerated with current model version

#### Scenario: mem_consolidate runs consolidation

- GIVEN project and optional dry_run flag
- WHEN mem_consolidate is called (admin only)
- THEN consolidation runs, results returned

#### Scenario: mem_synthesize creates capsule

- GIVEN topic and optional project
- WHEN mem_synthesize is called
- THEN KnowledgeCapsule is generated or updated

#### Scenario: mem_capsule_list returns all capsules

- GIVEN optional project filter
- WHEN mem_capsule_list is called
- THEN list of capsules with topic, confidence, version is returned

#### Scenario: mem_capsule_get returns full capsule

- GIVEN topic and optional project
- WHEN mem_capsule_get is called
- THEN full capsule with all fields is returned

#### Scenario: mem_antipatterns returns detected patterns

- GIVEN optional project and severity filter
- WHEN mem_antipatterns is called
- THEN list of anti-patterns with evidence and suggestions is returned

#### Scenario: mem_knowledge_boundary shows domain confidence

- GIVEN domain and optional project
- WHEN mem_knowledge_boundary is called
- THEN confidence level, evidence, and suggestions are returned

#### Scenario: mem_beliefs shows belief states

- GIVEN optional subject, state, project filters
- WHEN mem_beliefs is called
- THEN beliefs with current state, confidence, and history are returned

#### Scenario: mem_entities shows extracted entities

- GIVEN optional entity_type, query, project filters
- WHEN mem_entities is called
- THEN entities with aliases, properties, and observations are returned

#### Scenario: mem_principles shows abstract knowledge

- GIVEN optional project
- WHEN mem_principles is called
- THEN principles (highest compaction level) are returned

### Requirement: F2.75 Tools (2 tools)

#### Scenario: mem_inject builds smart context

- GIVEN task_description and optional current_files
- WHEN mem_inject is called
- THEN formatted context with memories, capsules, warnings, boundaries is returned

#### Scenario: mem_transfer suggests cross-project knowledge

- GIVEN new project and initial context
- WHEN mem_transfer is called
- THEN knowledge transfers from other projects are suggested with relevance scores

### Requirement: F3 Tools (3 tools — Multi-Agent)

#### Scenario: mem_share marks observation as shared

- GIVEN observation_id
- WHEN mem_share is called
- THEN observation scope changes to Project (visible to team)

#### Scenario: mem_team_capsule aggregates team knowledge

- GIVEN project
- WHEN mem_team_capsule is called
- THEN capsule aggregating observations from multiple agents is returned

#### Scenario: mem_agent_status shows per-agent knowledge

- GIVEN project
- WHEN mem_agent_status is called
- THEN knowledge boundaries and stats per agent are returned

### Requirement: MCP Annotations

The system MUST annotate tools with MCP metadata.

#### Scenario: Tools have title annotations

- GIVEN any MCP tool
- WHEN tool is listed
- THEN human-readable title is present

#### Scenario: Tools have hint annotations

- GIVEN mem_search
- WHEN inspected
- THEN read_only_hint=true

#### Scenario: Destructive tools have destructive_hint

- GIVEN mem_delete
- WHEN inspected
- THEN destructive_hint=true

### Requirement: MCP Resources [Innovation 8]

The system MUST expose 3 resources via MCP Resources protocol.

#### Scenario: current-context resource

- GIVEN project with observations
- WHEN read_resource("engram://project/current-context")
- THEN formatted smart context is returned

#### Scenario: knowledge-capsules resource

- GIVEN project with capsules
- WHEN read_resource("engram://project/knowledge-capsules")
- THEN list of all capsules formatted is returned

#### Scenario: anti-patterns resource

- GIVEN project with active anti-patterns
- WHEN read_resource("engram://project/anti-patterns")
- THEN list of active anti-patterns with suggestions is returned
