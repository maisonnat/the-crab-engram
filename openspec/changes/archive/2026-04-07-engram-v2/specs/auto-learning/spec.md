# Auto-Learning Specification

## Purpose

Sistema de auto-aprendizaje que consolida, sintetiza, detecta patrones, resuelve creencias, y compacta memorias. El sistema no solo recuerda — evoluciona.

## Requirements

### Requirement: Consolidation Engine

The system MUST periodically clean, merge, and improve the knowledge base.

#### Scenario: Semantic duplicates are merged

- GIVEN 2 observations with cosine_similarity > 0.92
- WHEN consolidation runs
- THEN observations are merged into one, preserving metadata from the most-accessed

#### Scenario: Obsolete observations are marked

- GIVEN observation A with supersedes edge from B
- WHEN consolidation runs
- THEN A is marked consolidation_state='obsolete'

#### Scenario: Contradictions are flagged

- GIVEN 2 observations with same topic_key but opposite sentiment (cosine < -0.3)
- WHEN consolidation runs
- THEN both are flagged consolidation_state='conflict'

#### Scenario: Patterns are extracted from similar bugfixes

- GIVEN 3+ bugfixes with cosine_similarity > 0.8
- WHEN consolidation runs
- THEN new observation type="pattern" is created with the common pattern

#### Scenario: Dry run does not modify data

- GIVEN consolidation called with dry_run=true
- WHEN consolidation completes
- THEN ConsolidationResult is returned but no data is modified

#### Scenario: Auto-consolidation triggers at threshold

- GIVEN project with 501 observations
- WHEN auto-consolidate timer fires
- THEN consolidation runs automatically

### Requirement: Knowledge Capsules

The system MUST synthesize dense knowledge summaries by topic.

#### Scenario: HeuristicSynthesizer generates capsule

- GIVEN 20 observations about "auth-architecture" topic
- WHEN HeuristicSynthesizer.synthesize is called
- THEN KnowledgeCapsule is created with summary, key_decisions, known_issues, anti_patterns, best_practices

#### Scenario: ChainedSynthesizer falls back to heuristic

- GIVEN LLM API unavailable
- WHEN ChainedSynthesizer.synthesize is called
- THEN HeuristicSynthesizer is used (no error)

#### Scenario: Capsule version increments on rebuild

- GIVEN capsule with version=2
- WHEN rebuild is called with new observations
- THEN version=3, summary is updated, confidence recalculated

#### Scenario: Search suggests capsule when >5 matches same topic

- GIVEN search returns 6 observations with topic="auth-architecture"
- WHEN results are formatted
- THEN suggestion "Knowledge capsule available: 'auth-architecture'" is shown

#### Scenario: Auto-synthesis on threshold

- GIVEN topic with >10 observations and no capsule
- WHEN consolidation runs
- THEN capsule is auto-created for that topic

### Requirement: Anti-Pattern Detection

The system MUST automatically detect when the project repeats errors.

#### Scenario: RecurringBug detected

- GIVEN 3+ bugfixes with cosine_similarity > 0.8
- WHEN anti-pattern detection runs
- THEN AntiPattern::RecurringBug is created with evidence and suggestion

#### Scenario: RevertPattern detected

- GIVEN graph cycle: A supersedes B supersedes A
- WHEN anti-pattern detection runs
- THEN AntiPattern::RevertPattern is created

#### Scenario: HotspotFile detected

- GIVEN file mentioned in >10 observations
- WHEN anti-pattern detection runs
- THEN AntiPattern::HotspotFile is created with severity

#### Scenario: Anti-patterns appear in session context

- GIVEN project with active anti-patterns
- WHEN session context is built
- THEN warnings are included at the top of context

### Requirement: Auto Graph Evolution

The system MUST detect relationships the agent didn't explicitly create.

#### Scenario: Temporal correlation creates CausedBy edge

- GIVEN observation A created before B in 3+ different sessions
- WHEN graph evolution runs
- THEN CausedBy edge A→B is created with auto_detected=true

#### Scenario: Search co-occurrence creates RelatedTo edge

- GIVEN observations X and Y appear together in 3+ searches
- WHEN graph evolution runs
- THEN RelatedTo edge X↔Y is created

#### Scenario: File correlation creates RelatedTo edge

- GIVEN 2 observations mentioning the same file path
- WHEN graph evolution runs
- THEN RelatedTo edge is created between them

#### Scenario: Auto-detected edges have auto_detected flag

- GIVEN any edge created by graph evolution
- WHEN edge is queried
- THEN auto_detected=true

### Requirement: Provenance Tracking

The system MUST track how each observation was verified.

#### Scenario: TestVerified gets high confidence

- GIVEN observation created with provenance_source=TestVerified
- WHEN stored
- THEN provenance_confidence=0.95

#### Scenario: LlmReasoning gets medium confidence

- GIVEN observation created without provenance specified
- WHEN stored
- THEN provenance_source=LlmReasoning, provenance_confidence=0.6 (default)

#### Scenario: Search filters by min_confidence

- GIVEN observations with various confidence levels
- WHEN searched with min_confidence=0.8
- THEN only observations with confidence >= 0.8 are returned

### Requirement: Spaced Repetition [Innovation 3]

The system MUST schedule periodic reviews of knowledge to detect and prevent forgetting.

#### Scenario: Schedule creates initial review

- GIVEN memory_id for a semantic memory
- WHEN schedule_review is called
- THEN review entry created with interval=1d, ease=2.5, next_review=now+1d

#### Scenario: Perfect review extends interval

- GIVEN review with interval=1d, ease=2.5
- WHEN process_review(memory_id, Perfect) is called
- THEN interval=2.5d (1 * 2.5)

#### Scenario: Forgotten review resets interval

- GIVEN review with interval=5d
- WHEN process_review(memory_id, Forgotten) is called
- THEN interval=1d, ease=2.3 (2.5 - 0.2)

#### Scenario: Cold start bootstraps reviews

- GIVEN project with 100 observations and 0 reviews
- WHEN bootstrap_reviews is called
- THEN 50 reviews are created with distributed intervals (top 10: 3d, 11-30: 1d, 31-50: 0.5d)

#### Scenario: Pending reviews appear in smart injection

- GIVEN review with next_review < now
- WHEN smart context is built
- THEN review appears as "refresh reminder"

### Requirement: Memory Compaction [Deep Investigation]

The system MUST compact memories through levels of abstraction.

#### Scenario: Raw to Fact compaction

- GIVEN 3+ observations on same topic
- WHEN compaction stage 1 runs
- THEN NewFact is created with summary and source_observations

#### Scenario: Fact to Pattern compaction

- GIVEN 3+ Facts in same domain with semantic similarity
- WHEN compaction stage 2 runs
- THEN NewPattern is created (observation type="pattern", provenance=Inferred 0.4)

#### Scenario: Pattern to Principle compaction

- GIVEN same Pattern appearing in 3+ different domains
- WHEN compaction stage 3 runs
- THEN NewPrinciple is created (observation type="principle", provenance=Inferred 0.3)

#### Scenario: SmartInjector selects correct level

- GIVEN query "how do we handle errors"
- WHEN SmartInjector determines compaction level
- THEN level=Pattern (trend question, not specific)

### Requirement: Agent Personality [Innovation 9]

The system MUST infer agent working style per project.

#### Scenario: Strength detected from observation types

- GIVEN agent with 80% bugfix observations
- WHEN personality is analyzed
- THEN strength: "debugging"

#### Scenario: Weakness detected from anti-patterns, not absence

- GIVEN agent with no testing observations
- WHEN personality is analyzed
- THEN testing is NOT marked as weakness (may not have had opportunity)

#### Scenario: Weakness from recurring anti-patterns

- GIVEN agent with 3 anti-patterns in CSS domain
- WHEN personality is analyzed
- THEN weakness: "css" (real evidence of failure)

#### Scenario: Style compatibility affects transfers

- GIVEN agent A (functional style) and agent B (functional style)
- WHEN cross-project transfer is suggested
- THEN style_compatibility is high, transfer ranked higher
