# Core Types Specification

## Purpose

Definir todos los tipos puros del dominio sin dependencias externas pesadas. Los tipos son serializables, clonables, y compatibles con el formato JSON de Engram Go.

## Requirements

### Requirement: Observation Type System

The system MUST represent every piece of memory as an `Observation` with strict typing.

#### Scenario: Create observation with all fields

- GIVEN valid ObservationType, Scope, title, content, session_id, project
- WHEN observation is created
- THEN all fields are populated: id, type, scope, title, content, session_id, project, topic_key, created_at, updated_at, access_count=0, last_accessed=None, pinned=false, normalized_hash

#### Scenario: ObservationType must be strict enum

- GIVEN any observation creation
- WHEN ObservationType is specified
- THEN it MUST be one of: Bugfix, Decision, Architecture, Pattern, Discovery, Learning, Config, Convention, ToolUse, FileChange, Command, FileRead, Search, Manual

#### Scenario: Scope isolation

- GIVEN observations with scope=Personal and scope=Project
- WHEN searched by a different agent
- THEN Personal observations are NOT visible to other agents

### Requirement: Session Lifecycle

The system MUST track sessions with start/end times and summaries.

#### Scenario: Session creation

- GIVEN a project name
- WHEN session is started
- THEN session is created with UUID v4, started_at timestamp, ended_at=None

#### Scenario: Session end with summary

- GIVEN an active session
- WHEN session is ended with summary text
- THEN ended_at is set and summary is persisted

### Requirement: Topic Key Generation

The system SHOULD generate topic keys using family heuristics compatible with Engram Go.

#### Scenario: Architecture observation gets architecture/* prefix

- GIVEN ObservationType::Architecture and title "auth-jwt-flow"
- WHEN topic_key is generated
- THEN result is "architecture/auth-jwt-flow"

#### Scenario: Slugify preserves alphanumeric and hyphens

- GIVEN text "Fix N+1 Query in UserList!"
- WHEN slugified
- THEN result is "fix-n1-query-in-userlist"

### Requirement: Temporal Knowledge Graph Edges

The system MUST represent relationships as temporal edges with validity windows.

#### Scenario: Edge creation with temporal columns

- GIVEN source_id, target_id, relation_type
- WHEN edge is created
- THEN valid_from=now, valid_until=None (vigente), superseded_by=None

#### Scenario: Edge auto-closure on new relation

- GIVEN existing edge A→B with relation=RelatedTo
- WHEN new edge A→B with relation=RelatedTo is created
- THEN old edge valid_until=now, new edge valid_from=now

#### Scenario: RelationType must be strict

- GIVEN any edge creation
- WHEN relation is specified
- THEN it MUST be one of: CausedBy, RelatedTo, Supersedes, Blocks, PartOf

### Requirement: Relevance Scoring

The system MUST compute a composite relevance score for search results.

#### Scenario: Decayed score decreases over time

- GIVEN observation created 30 days ago with no accesses
- WHEN decay_score is computed
- THEN score is ~50% of original (half-life 30 days)

#### Scenario: Pinned observation gets maximum recency score

- GIVEN pinned observation of any age
- WHEN decay_score is computed
- THEN recency component is 1.0

#### Scenario: Access frequency boosts score

- GIVEN observation accessed 20 times
- WHEN decay_score is computed
- THEN frequency component contributes positively (capped)

### Requirement: Episodic-Semantic Separation [Innovation 1]

The system MUST separate episodic memories (what happened) from semantic memories (what is known).

#### Scenario: Episodic memory captures temporal context

- GIVEN a session where a bug was fixed
- WHEN episodic memory is created
- THEN it includes session_id, timestamp, what_happened, context (where_, why, files_before as git hash), emotional_valence, surprise_factor

#### Scenario: Semantic memory tracks source episodes

- GIVEN episodic memory accessed 3+ times with surprise_factor > 0.5
- WHEN consolidated to semantic
- THEN semantic memory includes source_episodes pointing to original episode, domain, confidence

### Requirement: Emotional Salience [Innovation 2]

The system MUST infer emotional importance and modify decay accordingly.

#### Scenario: Frustration detected in content

- GIVEN observation content with "finally" + "hours" + "weird"
- WHEN salience is inferred
- THEN emotional_valence is negative (< 0)

#### Scenario: Achievement detected in content

- GIVEN observation content with "elegant" + "solved" + "breakthrough"
- WHEN salience is inferred
- THEN emotional_valence is positive (> 0)

#### Scenario: Salience modifies decay

- GIVEN observation with high positive valence and surprise
- WHEN final_score is computed
- THEN decay is slower than neutral observation (multiplier > 1.0)

### Requirement: Belief Resolution [Deep Investigation]

The system MUST resolve contradictions via state machine, not just flag them.

#### Scenario: New belief creation

- GIVEN subject "auth_method" with value "RS256"
- WHEN no existing belief for subject
- THEN belief is created with state=Active, confidence=0.5

#### Scenario: Confirming existing belief

- GIVEN existing belief "auth_method=RS256" confidence=0.5
- WHEN new evidence confirms same value
- THEN confidence increases, state may become Confirmed if >0.9

#### Scenario: Contesting with similar confidence

- GIVEN existing belief "auth_method=RS256" confidence=0.7
- WHEN new evidence says "auth_method=ES256" confidence=0.65
- THEN state=Contested, current_value unchanged, waiting for more evidence

#### Scenario: Updating with stronger evidence

- GIVEN existing belief "auth_method=RS256" confidence=0.5
- WHEN new evidence says "auth_method=ES256" confidence=0.85
- THEN state=Active, current_value=ES256, previous value preserved in history

#### Scenario: User retraction

- GIVEN any belief state
- WHEN user explicitly corrects
- THEN state=Retracted, correction becomes new current_value

### Requirement: Memory Lifecycle [Deep Investigation]

The system MUST apply different lifecycle policies per ObservationType.

#### Scenario: Decision is permanent

- GIVEN observation type=Decision
- WHEN lifecycle policy is applied
- THEN auto_delete_after_days=None, decay_multiplier=0.5, never auto-archived

#### Scenario: Command auto-purges after 6 months

- GIVEN observation type=Command created 181 days ago
- WHEN lifecycle transitions run
- THEN state=Deleted (auto-purge)

#### Scenario: Bugfix archived but preserved

- GIVEN observation type=Bugfix created 181 days ago
- WHEN lifecycle transitions run
- THEN state=Archived, NOT deleted (valuable for anti-patterns)

#### Scenario: FileRead is ephemeral

- GIVEN observation type=FileRead created 15 days ago
- WHEN lifecycle transitions run
- THEN state=Stale, searchable_when_stale=false

### Requirement: Knowledge Boundary Tracking [Innovation 6]

The system MUST track confidence levels per domain.

#### Scenario: Expert level after sufficient evidence

- GIVEN 25 observations in "rust" domain with high successful_applications
- WHEN boundary is evaluated
- THEN confidence_level=Expert

#### Scenario: Aware level with minimal evidence

- GIVEN 2 observations in "kubernetes" domain
- WHEN boundary is evaluated
- THEN confidence_level=Aware

#### Scenario: Failure lowers level

- GIVEN domain with confidence_level=Familiar
- WHEN failed_applications > successful_applications
- THEN confidence_level is lowered

### Requirement: Entity Resolution [Deep Investigation]

The system MUST resolve different textual references to the same entity.

#### Scenario: Person entity alias matching

- GIVEN "Alice es la CTO" and "our CTO approved the PR"
- WHEN entities are extracted
- THEN both resolve to same Person entity (canonical_name="Alice", aliases=["our CTO"])

#### Scenario: File entity from path and description

- GIVEN "src/auth.rs" and "the auth file"
- WHEN entities are extracted
- THEN both resolve to same File entity

#### Scenario: Entity-aware search finds by entity not just text

- GIVEN query "which vendors need templates" and entity "Vendor X" with alias "our main supplier" in observation "Vendor X requires PO format"
- WHEN entity-aware search runs
- THEN observation is found despite "templates" ≠ "format"
