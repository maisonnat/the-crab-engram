# Smart Context Specification

## Purpose

Inyectar el conocimiento correcto en el momento correcto, con awareness de conocimiento transferible entre proyectos y revisión periódica.

## Requirements

### Requirement: Smart Context Injection

The system MUST inject relevant context based on current task, not just recent observations.

#### Scenario: Task-relevant memories prioritized

- GIVEN task "implement JWT refresh" and 1000 observations
- WHEN context is built
- THEN top 5 memories most similar to task description are included (vector search)

#### Scenario: File history included for current files

- GIVEN current_files=["src/auth.rs"]
- WHEN context is built
- THEN up to 3 observations mentioning auth.rs are included

#### Scenario: Knowledge capsules included for relevant topics

- GIVEN task about authentication and capsule "auth-architecture" exists
- WHEN context is built
- THEN capsule is included (max 2 capsules)

#### Scenario: Token budget respected

- GIVEN context exceeds 2000 token budget
- WHEN context is trimmed
- THEN priority order is: warnings > boundaries > capsules > review_reminders > memories > file_history

#### Scenario: Anti-patterns shown as warnings

- GIVEN active RecurringBug anti-pattern
- WHEN context is built
- THEN warning is included at top of context

#### Scenario: Knowledge boundaries shown for current domains

- GIVEN task in "kubernetes" domain and boundary=Aware
- WHEN context is built
- THEN "Estás entrando en territorio kubernetes — tu experiencia aquí es `Aware`" is shown

### Requirement: Cross-Project Learning

The system MUST suggest knowledge from other projects when starting new work.

#### Scenario: Relevant capsule suggested from other project

- GIVEN project A has capsule "JWT auth" and new project B has README mentioning "authentication"
- WHEN suggest_prior_knowledge is called
- THEN capsule from A is suggested with relevance > 0.7

#### Scenario: Low relevance filtered out

- GIVEN capsule about "kubernetes deployment" and new project about "Rust CLI"
- WHEN suggest_prior_knowledge is called
- THEN capsule is NOT suggested (relevance < 0.7)

#### Scenario: Auto-transfer on new project

- GIVEN project with <10 observations
- WHEN session starts
- THEN transfer suggestions are shown automatically

#### Scenario: Transfer acceptance tracked

- GIVEN agent uses a suggested transfer
- WHEN recorded
- THEN knowledge_transfers entry shows accepted=1

### Requirement: MCP Resources [Innovation 8]

The system MUST expose memory as MCP Resources (push) in addition to Tools (pull).

#### Scenario: Three standard resources available

- GIVEN MCP server is running
- WHEN list_resources is called
- THEN 3 resources returned: current-context, knowledge-capsules, anti-patterns

#### Scenario: Current-context resource returns formatted context

- GIVEN project with observations and capsules
- WHEN read_resource("engram://project/current-context") is called
- THEN formatted Markdown context is returned

#### Scenario: Notifications are granular

- GIVEN new knowledge capsule created during consolidation
- WHEN notification is sent
- THEN only knowledge-capsules resource subscribers are notified (not current-context)

#### Scenario: Notifications are batched for current-context

- GIVEN 5 observations saved in 2 minutes
- WHEN notifications are sent
- THEN only 1 batched notification for current-context (not 5 individual)

### Requirement: Export Context

The system MUST generate a standalone system prompt from project knowledge.

#### Scenario: Export-context generates Markdown

- GIVEN project with capsules, observations, and anti-patterns
- WHEN export-context is called
- THEN Markdown file with top capsules, top observations, warnings, and boundaries is generated

#### Scenario: Export-context respects token limit

- GIVEN export-context with --max-tokens 2000
- WHEN generated
- THEN output is under 2000 tokens

#### Scenario: Export-context works without F2 features

- GIVEN only F1 features available (no embeddings, no capsules)
- WHEN export-context is called
- THEN output is generated from observations and stats only (no crash)
