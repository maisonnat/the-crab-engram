# Search Engine Specification

## Purpose

Motor de búsqueda que evoluciona de FTS5 baseline a hybrid search (FTS5 + vector) con búsqueda type-aware y entity-aware.

## Requirements

### Requirement: FTS5 Baseline Search

The system MUST provide keyword-based full-text search via FTS5.

#### Scenario: Keyword match returns relevant results

- GIVEN observations containing "JWT auth with RS256"
- WHEN searched for "JWT"
- THEN matching observation appears in results

#### Scenario: Multi-word query with ranking

- GIVEN multiple observations mentioning "auth" in different contexts
- WHEN searched for "auth configuration"
- THEN results ranked by relevance (title match > content match)

#### Scenario: Filter by type and project

- GIVEN observations of various types across projects
- WHEN searched with type=Bugfix and project=engram-rust
- THEN only Bugfix observations from engram-rust are returned

### Requirement: Hybrid Search with RRF

The system MUST combine FTS5 and vector search using Reciprocal Rank Fusion.

#### Scenario: Semantic match found via vectors

- GIVEN observation about "N+1 query fix"
- WHEN searched for "performance issue"
- THEN observation appears (semantic similarity, not keyword match)

#### Scenario: Hybrid outperforms FTS5 alone

- GIVEN test corpus of 1000 observations
- WHEN hybrid search is compared to FTS5-only
- THEN NDCG@10 improvement > 20%

#### Scenario: Fallback to FTS5 when embedder unavailable

- GIVEN embedder fails to initialize
- WHEN search is performed
- THEN results come from FTS5-only (no crash)

### Requirement: Type-Aware Search [Innovation 1]

The system MUST differentiate between episodic and semantic memory searches.

#### Scenario: "What happened" query targets episodic

- GIVEN query "what happened with the auth bug last week"
- WHEN query type is classified
- THEN target=Episodic, search focuses on episodic_memories table

#### Scenario: "What is" query targets semantic

- GIVEN query "what is the auth configuration"
- WHEN query type is classified
- THEN target=Semantic, search focuses on semantic_memories table

#### Scenario: Generic query searches both

- GIVEN query "auth"
- WHEN query type is classified
- THEN target=Both, results merged from both tables

### Requirement: Entity-Aware Search [Deep Investigation]

The system MUST resolve entities in queries to improve recall.

#### Scenario: Entity resolution improves recall

- GIVEN query "which vendors need templates" and entity "Vendor X" with observation "Vendor X requires PO format"
- WHEN entity-aware search runs
- THEN observation found despite "templates" ≠ "format" (entity match)

#### Scenario: Triple strategy merge

- GIVEN entity-aware search
- WHEN results are collected
- THEN results from vector search, FTS search, and entity lookup are merged and reranked

### Requirement: Search Result Enrichment

The system SHOULD enrich search results with graph context.

#### Scenario: Related observations included

- GIVEN search result observation with 2 active edges
- WHEN results are formatted
- THEN 1-2 related observations are included as context

#### Scenario: Belief states shown in results

- GIVEN observation about "auth uses RS256" with belief state=Confirmed
- WHEN results are formatted
- THEN "(Confirmed, 3 sources)" annotation is shown

### Requirement: Search Logging for Graph Evolution

The system SHOULD log search result co-occurrence for graph evolution.

#### Scenario: Co-occurrence tracked

- GIVEN observations X and Y appear together in 3 searches
- WHEN graph evolution runs
- THEN RelatedTo edge between X and Y is created
