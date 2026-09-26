# Epistemic Knowledge Runtime
## Conceptual System Design

**Status:** Conceptual architecture  
**Purpose:** Define a self-maintaining epistemic knowledge system that continuously ingests observations, interprets them into structured knowledge, validates and integrates trustworthy knowledge into a canonical core, incubates knowledge that does not yet fit, evolves its ontology under controlled rules, and forgets or compacts data over time.

---

# 1. Executive Summary

This document specifies a conceptual architecture for an **Epistemic Knowledge Runtime**: a continuously operating system that transforms heterogeneous observations into structured, typed, provenance-aware, revisable knowledge.

The system is not merely a knowledge graph. A graph is one of its storage and representation mechanisms. The larger system defines a complete **lifecycle of knowing**:

1. observe,
2. interpret,
3. propose,
4. resolve,
5. validate,
6. integrate,
7. depend upon,
8. revise,
9. consolidate,
10. forget,
11. investigate again.

The architecture distinguishes strongly between:

- **observed information** and **accepted knowledge**,
- **candidate assertions** and **canonical assertions**,
- **transient interpretations** and **integrated knowledge**,
- **confidence** and **validity**,
- **schema compatibility** and **semantic usefulness**,
- **semantic retraction** and **physical deletion**.

The system has a strongly governed **Canonical Core** surrounded by an **Incubation Forest**. New information enters through an **Observation Layer**, is transformed by agent-driven interpretation, and may either be integrated into the Canonical Core or parked in transient graph roots where it can accumulate structure until later passes make integration possible.

A central principle is:

> Failure to integrate is itself information.

Repeated patterns in non-canonical knowledge may reveal missing concepts, missing edge types, insufficient constraints, or gaps in the ontology. The system can therefore propose schema evolution from accumulated evidence, while keeping schema changes subject to stronger validation than ordinary knowledge changes.

The system is best thought of as a **knowledge runtime** with an explicit epistemic boundary and a continuously executing **Epistemic Loop**.

---

# 2. Core Concept

The runtime maintains a structured answer to the question:

> What does the system currently know, why does it believe it, how may other parts of the system depend on it, and under what conditions should that knowledge be revised or forgotten?

The system should never conflate all ingested data with truth.

At minimum, it distinguishes:

```text
bytes
  ↓
observation
  ↓
interpretation
  ↓
candidate assertion
  ↓
validated assertion
  ↓
integrated canonical knowledge
```

Each transition has different semantics, guarantees, and retention policies.

The primary abstraction is not the graph itself.

The primary abstraction is the state transition:

```text
State
  + Observation
  → Interpretation
  → Proposal
  → Validation
  → Integration / Parking
  → Maintenance
  → New State
```

Formally:

```text
Sₙ₊₁ = F(Sₙ, Oₙ)
```

Where:

- `Sₙ` is the complete epistemic state at revision `n`,
- `Oₙ` is a set of new observations,
- `F` is the governed epistemic transition process,
- `Sₙ₊₁` is the next committed state.

---

# 3. What the System Is

The system is a combination of several architectural ideas.

## 3.1 An epistemic system

It represents not just facts, but epistemic status:

- observed,
- interpreted,
- proposed,
- corroborated,
- disputed,
- accepted,
- superseded,
- retracted,
- stale,
- archived.

It therefore models both knowledge and the system's relationship to that knowledge.

## 3.2 A knowledge runtime

It continuously executes operations over knowledge:

- entity resolution,
- type checking,
- reference validation,
- provenance verification,
- contradiction detection,
- schema validation,
- integration planning,
- schema migration,
- consolidation,
- expiry,
- garbage collection.

The runtime analogy is intentional.

| Programming runtime | Knowledge runtime |
|---|---|
| values | entities / assertions |
| types | ontology |
| references | edges |
| heap | knowledge fabric |
| type checker | schema validator |
| garbage collector | retention / reclamation |
| compiler inference | schema inference |
| event loop | epistemic loop |
| exceptions | contradictions / invalid states |
| immutable log | revision history |

## 3.3 A self-maintaining graph system

The system does not rely on humans to manually curate every new piece of information.

Agents may:

- identify missing knowledge,
- gather observations,
- infer entities and relations,
- resolve references,
- propose assertions,
- validate assertions,
- detect conflicts,
- propose schema extensions,
- reconsider parked knowledge,
- identify obsolete material.

However, agents are not privileged writers to canonical state.

## 3.4 A self-structuring knowledge system

Knowledge is required to conform to the ontology, but recurring non-conforming knowledge may provide evidence that the ontology itself is insufficient.

Therefore the relationship is bidirectional:

```text
ontology constrains knowledge

and

observed knowledge pressures ontology to evolve
```

Schema evolution is controlled and explicit rather than implicit.

---

# 4. What the System Is Not

## 4.1 Not merely a knowledge graph

A graph database can store nodes and edges, but it does not by itself define:

- how observations become trusted facts,
- provenance requirements,
- validation gates,
- transient knowledge spaces,
- schema discovery,
- forgetting,
- retraction,
- agent governance.

The graph is a representation substrate, not the whole system.

## 4.2 Not merely an ontology

The ontology is the type and semantic constraint system used by the runtime.

It is one layer within the larger epistemic architecture.

## 4.3 Not merely RAG

Retrieval-augmented generation may be used by agents, but RAG does not inherently provide:

- canonicality,
- durable identity,
- referential integrity,
- temporal semantics,
- contradiction handling,
- graph integration,
- schema evolution.

## 4.4 Not merely an agent swarm

Agents are replaceable workers.

The integrity of the system must not depend on any single model or prompt behaving correctly.

The runtime remains valid even if agent implementations change.

## 4.5 Not merely AI memory

Memory implies persistence and recall.

This system also performs:

- epistemic classification,
- semantic integration,
- validation,
- revision,
- ontology evolution,
- consolidation,
- forgetting.

---

# 5. Terminology

## 5.1 Knowledge Runtime

The entire software system.

It coordinates ingestion, agents, validation, graph management, ontology management, maintenance, storage, and retrieval.

## 5.2 Knowledge Fabric

The complete persisted knowledge substrate.

It may contain many graph roots, stores, schemas, histories, mappings, and evidence collections.

## 5.3 Canonical Core

The main strongly validated graph.

Knowledge in the Canonical Core satisfies the system's highest integrity requirements and may safely be depended upon by canonical computations.

## 5.4 Incubation Forest

The collection of non-canonical graph roots containing interpretations or structures that have not yet met canonical integration requirements.

These graph roots may be internally coherent and strongly typed according to local schemas without being accepted by the canonical ontology.

## 5.5 Observation Layer

Immutable records of external information entering the runtime.

Examples:

- documents,
- Slack message diffs,
- email messages,
- API responses,
- Git commits,
- database changes,
- crawler results,
- human input.

## 5.6 Epistemic Loop

The continuous process through which the runtime:

- discovers,
- observes,
- interprets,
- validates,
- integrates,
- revisits,
- consolidates,
- forgets.

## 5.7 Integration

The controlled derivation and commitment of canonical knowledge from non-canonical knowledge.

Integration does not destroy the source interpretation.

## 5.8 Consolidation

Reduction of redundant or intermediate representations while preserving useful knowledge and required provenance.

## 5.9 Retraction

Semantic removal of a canonical assertion from the active world view without necessarily destroying historical evidence.

## 5.10 Physical Reclamation

Actual deletion of persisted bytes after they are no longer required by retention, provenance, audit, legal, or system constraints.

---

# 6. Fundamental Invariants

The following invariants define the integrity model.

## 6.1 Canonical dependency invariant

> Canonical knowledge may depend only on canonical knowledge or retained admissible evidence.

A canonical object may never depend directly on an unresolved transient object.

Allowed:

```text
Transient → Canonical
Transient → Transient
Canonical → Canonical
```

Forbidden:

```text
Canonical → unresolved Transient
```

This creates a one-way integrity membrane.

## 6.2 No dangling references

Every committed reference must resolve to an existing object within its allowed dependency domain.

Examples:

```text
NodeRef → existing Node
Edge.source → existing Node
Edge.target → existing Node
TypeRef → existing Type
PropertyRef → existing Property
EvidenceRef → existing Evidence
```

## 6.3 Type validity

Every committed value must satisfy its declared type.

Every relation must satisfy its source and target type constraints.

## 6.4 Stable identity

Human-readable names are not identities.

All persistent entities, types, properties, edges, assertions, schemas, and evidence objects use stable IDs.

Renaming must not change identity.

## 6.5 Provenance

Every canonical assertion must have sufficient provenance according to policy.

"An agent said so" is not sufficient provenance.

## 6.6 Confidence is not validity

Agent confidence is metadata.

It does not bypass deterministic or policy-driven validation.

## 6.7 No direct agent mutation

Agents cannot directly mutate the Canonical Core.

They can only propose transactions.

## 6.8 Immutable historical transitions

Committed revisions are immutable.

Later changes produce new revisions, retractions, supersessions, or migrations.

## 6.9 Schema changes are transactions

Ontology evolution occurs through explicit schema transactions and stronger review gates.

## 6.10 A transaction cannot validate itself

The same actor or process that proposes a transaction cannot be the sole basis for its validation.

---

# 7. High-Level Architecture

```text
                     External World
                           │
             ┌─────────────┼─────────────┐
             │             │             │
          Slack          Docs          APIs
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                 ┌───────────────────┐
                 │ Observation Layer │
                 └─────────┬─────────┘
                           ▼
                 ┌───────────────────┐
                 │ Interpretation    │
                 │ Agents            │
                 └─────────┬─────────┘
                           ▼
                 ┌───────────────────┐
                 │ Candidate Graphs  │
                 │ / Assertions      │
                 └─────────┬─────────┘
                           ▼
                 ┌───────────────────┐
                 │ Resolution        │
                 │ + Validation      │
                 └─────────┬─────────┘
                           ▼
                ┌──────────┴──────────┐
                │                     │
                ▼                     ▼
       ┌─────────────────┐   ┌──────────────────┐
       │ Canonical Core  │   │ Incubation Forest│
       └────────┬────────┘   └─────────┬────────┘
                │                      │
                │         structure / │
                │          conflicts  │
                │          / patterns │
                │                      ▼
                │             Schema Proposals
                │                      │
                └──────────────┬───────┘
                               ▼
                      Ontology Evolution
                               │
                               ▼
                     Re-integration Pass
                               │
                               ↺
```

A maintenance path runs alongside all active processing:

```text
reconcile
  ↓
compress
  ↓
expire
  ↓
archive
  ↓
garbage collect
```

---

# 8. The Seed

The system begins from a minimal trusted seed.

The seed is not expected to contain a complete world ontology.

It contains only enough structure to begin safely extending knowledge.

Conceptually:

```rust
pub struct Seed {
    pub kernel: Kernel,
    pub ontology: Ontology,
    pub validators: Vec<ValidatorSpec>,
    pub agent_roles: Vec<AgentRole>,
    pub initial_assertions: Vec<Assertion>,
}
```

The seed should be as small as practical.

A smaller seed reduces the trusted computing base.

---

# 9. Trusted Kernel

The kernel contains rules that are not casually mutable by the ordinary knowledge loop.

Conceptually:

```rust
pub struct Kernel {
    pub identity_rules: IdentityRules,
    pub reference_rules: ReferenceRules,
    pub transaction_rules: TransactionRules,
    pub validation_rules: ValidationRules,
    pub provenance_rules: ProvenanceRules,
}
```

The kernel defines:

- what constitutes identity,
- how references resolve,
- what makes a transaction structurally valid,
- what validators must run,
- which dependencies are permitted,
- which state transitions are legal,
- how revision lineage works.

The kernel should not encode domain ontology such as "Person", "Project", or "Company".

Those belong to evolvable graph state.

---

# 10. Identity Model

Stable identity is central.

Suggested ID classes:

```rust
pub struct NodeId(pub u128);
pub struct EdgeId(pub u128);
pub struct TypeId(pub u128);
pub struct PropertyId(pub u128);
pub struct AssertionId(pub u128);
pub struct EvidenceId(pub u128);
pub struct ObservationId(pub u128);
pub struct AgentId(pub u128);
pub struct SchemaVersionId(pub u128);
pub struct GraphRootId(pub u128);
pub struct TransactionId(pub u128);
```

Production implementations may use UUIDv7, content-derived identifiers where appropriate, or another stable scheme.

Human-readable names remain attributes:

```rust
pub struct Node {
    pub id: NodeId,
    pub canonical_name: String,
    // ...
}
```

Identity must survive:

- renames,
- aliases,
- schema migration,
- merges,
- external source changes.

---

# 11. Ontology and Type System

The ontology defines the semantic type system of a graph.

## 11.1 Node types

```rust
pub struct NodeType {
    pub id: TypeId,
    pub name: String,
    pub parents: BTreeSet<TypeId>,
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    pub abstract_type: bool,
}
```

## 11.2 Property definitions

```rust
pub struct PropertyDefinition {
    pub id: PropertyId,
    pub name: String,
    pub value_type: ValueType,
    pub cardinality: Cardinality,
    pub required: bool,
    pub constraints: Vec<Constraint>,
}
```

## 11.3 Typed values

Values should not default to arbitrary JSON.

```rust
pub enum ValueType {
    String,
    Boolean,
    Integer,
    Float,
    Decimal,
    Timestamp,
    Duration,

    NodeRef {
        allowed_types: BTreeSet<TypeId>,
    },

    Enum {
        variants: BTreeSet<String>,
    },

    List(Box<ValueType>),

    Record(BTreeMap<String, ValueType>),
}
```

Runtime values mirror the declared type:

```rust
pub enum Value {
    String(String),
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Decimal(String),
    Timestamp(i64),
    Duration(i64),
    NodeRef(NodeId),
    Enum(String),
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
}
```

This prevents ambiguous representations.

For example:

```text
employer = "OpenAI"
```

is semantically weaker than:

```text
employer = NodeRef(organization/openai)
```

The second representation can be reference-checked and type-checked.

---

# 12. Typed Edges

Relations have schemas just like nodes.

```rust
pub struct EdgeType {
    pub id: TypeId,
    pub name: String,

    pub source_types: BTreeSet<TypeId>,
    pub target_types: BTreeSet<TypeId>,

    pub cardinality: EdgeCardinality,

    pub properties: BTreeMap<PropertyId, PropertyDefinition>,

    pub inverse: Option<TypeId>,
    pub symmetric: bool,
    pub transitive: bool,
}
```

A concrete edge:

```rust
pub struct Edge {
    pub id: EdgeId,
    pub type_id: TypeId,
    pub source: NodeId,
    pub target: NodeId,
    pub properties: BTreeMap<PropertyId, PropertyValue>,
    pub provenance: Vec<EvidenceId>,
    pub validation: ValidationState,
}
```

An edge may only commit if:

```text
source exists
target exists
edge type exists
source.type is compatible with edge.source_types
target.type is compatible with edge.target_types
edge properties satisfy schema
all required references resolve
```

---

# 13. Assertions as a Fundamental Primitive

The most general form of knowledge is an assertion.

```rust
pub struct Assertion {
    pub id: AssertionId,

    pub subject: Subject,
    pub predicate: Predicate,
    pub object: Object,

    pub evidence: BTreeSet<EvidenceId>,
    pub proposed_by: AgentId,

    pub validation: ValidationState,

    pub valid_time: TemporalRange,
    pub transaction_time: TemporalRange,
}
```

Possible subjects:

```rust
pub enum Subject {
    Node(NodeId),
    Edge(EdgeId),
    Type(TypeId),
}
```

Predicates:

```rust
pub enum Predicate {
    Property(PropertyId),
    Relation(TypeId),
}
```

Objects:

```rust
pub enum Object {
    Value(Value),
    Node(NodeId),
    Type(TypeId),
}
```

This allows node properties, relations, metadata, and schema-level claims to share a common provenance and lifecycle model.

---

# 14. Temporal Model

The Canonical Core should support bitemporal semantics where useful.

Two time dimensions are distinct.

## 14.1 Valid time

When the assertion was true in the represented world.

## 14.2 Transaction time

When the system believed or stored the assertion.

Example:

```text
Alice CEO_OF Acme
valid_time: 2024-01-01 → 2026-03-12
transaction_time: 2024-01-02 → present history
```

A later fact can supersede the active world view without deleting historical knowledge.

---

# 15. Observations

Observations represent what the system received from the external world.

They should be immutable.

```rust
pub struct Observation {
    pub id: ObservationId,
    pub source: SourceRef,
    pub content_hash: ContentHash,
    pub captured_at: Timestamp,
    pub content: ObservationContent,
}
```

Possible forms:

```rust
pub enum ObservationContent {
    Document(Document),
    ApiResponse(StructuredValue),
    DatabaseRecord(StructuredValue),
    FeedItem(Document),
    GraphFragment(Vec<ExternalTriple>),
    MessageBatch(MessageBatch),
    GitDiff(GitDiff),
}
```

An observation says:

> These bytes or structured values were observed from this source at this time.

It does not say:

> The semantic claims contained within them are true.

---

# 16. Evidence

Evidence is structured and independently referenceable.

```rust
pub struct Evidence {
    pub id: EvidenceId,
    pub source: EvidenceSource,
    pub content_hash: [u8; 32],
    pub extracted_by: AgentId,
    pub timestamp: i64,
    pub confidence: Confidence,
}
```

Possible source kinds:

```rust
pub enum EvidenceSource {
    Url(String),

    Document {
        document_id: String,
        section: Option<String>,
    },

    DatabaseRecord {
        database: String,
        table: String,
        key: String,
    },

    GraphAssertion(AssertionId),

    Observation(ObservationId),

    HumanStatement {
        identity: Option<String>,
    },
}
```

Evidence should preserve enough information to audit or reproduce the derivation where policy requires it.

---

# 17. Validation State

Validation should not be represented as a single boolean.

```rust
pub enum ValidationState {
    Proposed,

    Validating {
        completed: u32,
        required: u32,
    },

    Accepted {
        validators: BTreeSet<AgentId>,
    },

    Rejected {
        reasons: Vec<ValidationIssue>,
    },

    Disputed {
        competing_assertions: Vec<AssertionId>,
    },

    Superseded {
        by: AssertionId,
    },

    Retracted {
        reason: RetractionReason,
    },
}
```

Validation state is explicit and inspectable.

---

# 18. Agents

Agents are runtime workers, not canonical authorities.

```rust
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub capabilities: BTreeSet<Capability>,
    pub trust: TrustProfile,
}
```

Capabilities may include:

```rust
pub enum Capability {
    ExtractFacts,
    ResolveEntities,
    ValidateReferences,
    ValidateTypes,
    ValidateOntology,
    DetectContradictions,
    VerifyEvidence,
    DiscoverSchema,
    ApproveSchemaChange,
    PlanCrawl,
    ConsolidateKnowledge,
}
```

Agents can be implemented by:

- language models,
- deterministic code,
- external services,
- rules engines,
- human reviewers,
- hybrid systems.

The architecture should not depend on the implementation.

---

# 19. Transaction Model

No agent receives direct canonical mutation privileges.

Agents propose graph transactions.

```rust
pub struct GraphTransaction {
    pub id: TransactionId,
    pub proposer: AgentId,
    pub operations: Vec<GraphOperation>,
    pub evidence: BTreeSet<EvidenceId>,
}
```

Operations may include:

```rust
pub enum GraphOperation {
    CreateNode(NodeDraft),
    UpdateProperty(PropertyMutation),
    CreateEdge(EdgeDraft),
    DeleteEdge(EdgeId),

    AddAssertion(Assertion),
    RetractAssertion(AssertionId),

    DefineNodeType(NodeType),
    DefineEdgeType(EdgeType),
    ModifyProperty(PropertyDefinition),

    MergeEntity(EntityMerge),
}
```

Validation produces a distinct type:

```rust
pub struct ValidatedTransaction {
    pub tx: GraphTransaction,
    pub validated_against_revision: u64,
    pub validation_hash: [u8; 32],
}
```

Only a `ValidatedTransaction` may be committed.

This uses Rust's type system as part of the integrity boundary.

---

# 20. Validation Pipeline

The validation pipeline should combine deterministic checks and policy checks.

```rust
pub trait Validator {
    fn validate(
        &self,
        graph: &GraphSnapshot,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>>;
}
```

Typical validators:

1. structural validator,
2. reference validator,
3. type validator,
4. cardinality validator,
5. ontology constraint validator,
6. provenance validator,
7. authorization validator,
8. contradiction validator,
9. temporal consistency validator,
10. policy validator.

The deterministic subset should be rerunnable independently of AI agents.

---

# 21. Canonical Core

The Canonical Core contains integrated knowledge that has crossed the system's highest integrity boundary.

Characteristics:

- strongly typed,
- schema valid,
- referentially complete,
- provenance compliant,
- transactionally committed,
- revisioned,
- queryable as dependable state.

A simple representation:

```rust
pub struct CanonicalGraph {
    pub revision: u64,
    pub ontology: Ontology,
    pub nodes: BTreeMap<NodeId, Node>,
    pub edges: BTreeMap<EdgeId, Edge>,
    pub assertions: BTreeMap<AssertionId, Assertion>,
    pub evidence: BTreeMap<EvidenceId, Evidence>,
}
```

The production representation may be distributed across multiple stores.

The conceptual guarantee matters more than the physical format.

---

# 22. The Incubation Forest

Information that cannot yet be represented canonically should not be discarded and should not be forced into an inappropriate schema.

Instead it enters a non-canonical graph root.

Examples of why integration might fail:

- unknown entity identity,
- insufficient provenance,
- missing relation type,
- unsupported property,
- unresolved contradiction,
- ambiguous semantics,
- ontology mismatch,
- insufficient validation.

Each transient root may use its own stable schema.

Example:

```text
Transient Slack Schema
    SlackMessage
    Thread
    Reaction
    TentativeAction
    SocialCommitment
    Mention
```

The canonical ontology might only contain:

```text
Person
Project
Task
Decision
```

The transient schema can still be stable and internally typed without being canonical.

---

# 23. Knowledge Spaces

The architecture should distinguish knowledge domains explicitly.

```rust
pub enum KnowledgeSpace {
    Canonical,
    Transient(TransientRootId),
}
```

At the Rust API level, stronger typing is preferred:

```rust
pub struct CanonicalGraph {
    root: GraphRoot,
}

pub struct TransientGraph {
    root: GraphRoot,
}
```

References can encode dependency restrictions:

```rust
pub struct CanonicalRef<T> {
    id: NodeId,
    _marker: PhantomData<T>,
}

pub enum TransientRef<T> {
    Canonical(CanonicalRef<T>),
    Local(LocalRef<T>),
}
```

No equivalent `CanonicalGraph -> TransientRef` should exist.

Invalid dependency states should be difficult or impossible to represent.

---

# 24. Interpretation of Incoming Data

Consider a dump of recent Slack messages.

The initial flow is:

```text
Slack diff
   ↓
Observation
   ↓
TransientRoot
   ↓
Message nodes
   ↓
Entity candidates
   ↓
Action candidates
   ↓
Relations
```

Interpretation is intentionally separated from integration.

Example message:

```text
"Sarah said deployment should move to Tuesday."
```

Possible transient interpretation:

```text
Message
  ├── speaker → SarahCandidate
  ├── mentions → DeploymentCandidate
  └── proposes
       └── RescheduleCandidate
             └── date → Tuesday
```

The runtime then attempts resolution and integration.

If all required identities and schema mappings exist, a canonical transaction can be proposed.

If not, the interpretation remains parked.

---

# 25. Failure to Integrate as Information

A major architectural principle is:

> Repeated integration failure can reveal missing ontology.

Example transient observations:

```text
Project A → health "amber"
Project B → health "green"
Project C → health "red"
Project D → health "amber"
```

Suppose the canonical ontology has no project health concept.

The runtime should not immediately create an arbitrary canonical field.

Instead, transient roots accumulate evidence.

A later schema-discovery pass may infer a recurring structure:

```text
Project
  └── project-health
        ├── green
        ├── amber
        └── red
```

This can produce a schema proposal.

---

# 26. Schema Evolution

Schema evolution uses the same transactional philosophy as ordinary knowledge, but with stronger gates.

Example proposal:

```rust
pub struct SchemaProposal {
    pub source_roots: Vec<GraphRootId>,
    pub operations: Vec<SchemaOperation>,
    pub supporting_evidence: Vec<EvidenceId>,
    pub migration_plan: Option<MigrationPlan>,
}
```

A proposal might:

- add a type,
- add an edge type,
- add a property,
- relax or tighten a constraint,
- define a subtype,
- define a mapping from transient schema to canonical schema.

Schema validation may require:

- multiple independent signals,
- compatibility analysis,
- migration analysis,
- collision detection,
- proof that existing canonical state remains valid,
- human approval for high-impact changes.

---

# 27. Integration Plans

Integration should be explicit.

A transient graph is not copied wholesale into the Canonical Core.

Instead the runtime derives an integration plan.

```rust
pub struct IntegrationPlan {
    pub source_root: TransientRootId,
    pub source_schema: SchemaVersionId,
    pub target_schema: SchemaVersionId,

    pub mappings: Vec<Mapping>,
    pub transactions: Vec<CanonicalTransaction>,
}
```

An integration mapping may be conditional.

Example:

```text
Slack.TentativeAction
    → Canonical.Task

only if:
    actor identity resolves
    action is explicit enough
    action has not been superseded
    project reference resolves
    task schema requirements are satisfied
```

This makes integration resemble a semantic compiler.

---

# 28. Integration Does Not Move Data

When knowledge is integrated, the source interpretation is not conceptually "moved" into the canonical graph.

The system creates a canonical derivation.

```text
Observation
   ↓
Transient Interpretation
   ↓
Integration Plan
   ↓
Canonical Assertion
```

The provenance chain remains inspectable.

Example:

```text
Canonical Task #791
   ↑ derived_from
Integration #221
   ↑ interpreted_from
Transient Node #882
   ↑ extracted_from
Slack Observation #991
```

This makes schema changes, audits, and reprocessing possible.

---

# 29. Knowledge Lifecycle

A useful lifecycle is:

```text
RAW
  ↓
OBSERVED
  ↓
INTERPRETED
  ↓
INCUBATING
  ↓
INTEGRATABLE
  ↓
INTEGRATED
```

Alternate terminal states include:

```text
REJECTED
ARCHIVED
EXPIRED
```

Possible representation:

```rust
pub enum KnowledgeState {
    Observation,
    Transient,
    Incubating,
    Integratable,
    Integrated,
    Rejected,
    Archived,
    Expired,
}
```

These are lifecycle states, not separate truth systems.

At the highest integrity level, the system still has two trust domains:

```text
Canonical
NonCanonical
```

---

# 30. Knowledge Crawler

The crawler is the sensory acquisition subsystem.

It should not directly write facts.

It emits observations.

```rust
pub trait KnowledgeCrawler {
    async fn crawl(
        &self,
        ctx: &CrawlContext,
    ) -> Result<Vec<Observation>, CrawlError>;
}
```

Crawler sources may include:

- web,
- APIs,
- internal databases,
- Slack,
- email,
- Git,
- filesystems,
- feeds,
- event streams,
- external graphs.

---

# 31. Semantic Frontier

A conventional crawler operates from a URL frontier.

This runtime should operate from a **knowledge frontier**.

```rust
pub struct KnowledgeFrontier {
    pub tasks: Vec<KnowledgeTask>,
}
```

Tasks may include:

```rust
pub enum KnowledgeTask {
    DiscoverEntity {
        query: String,
    },

    FillProperty {
        entity: NodeId,
        property: PropertyId,
    },

    VerifyAssertion {
        assertion: AssertionId,
    },

    ResolveConflict {
        assertions: Vec<AssertionId>,
    },

    DiscoverRelations {
        entity: NodeId,
        relation_type: TypeId,
    },

    ExpandTopic {
        concept: NodeId,
    },

    RefreshStaleKnowledge {
        entity: NodeId,
    },

    RevisitTransientRoot {
        root: GraphRootId,
    },
}
```

The graph itself therefore drives future observation.

The crawler asks:

- what do we not know?
- what is weakly supported?
- what is stale?
- what is disputed?
- what repeatedly fails integration?
- which unresolved transient structures look valuable?

---

# 32. Frontier Prioritization

Crawl work should be ranked by expected value.

Possible factors:

```rust
pub struct FrontierItem {
    pub task: KnowledgeTask,
    pub importance: f32,
    pub uncertainty: f32,
    pub staleness: f32,
    pub expected_information_gain: f32,
    pub integration_potential: f32,
    pub estimated_cost: f32,
}
```

A conceptual priority function:

```text
priority =
    importance
  × uncertainty
  × information_gain
  × freshness_need
  × integration_potential
  ÷ expected_cost
```

The exact function is a policy choice.

---

# 33. The Epistemic Loop

The system continuously executes:

```text
canonical state
      ↓
detect gaps / stale knowledge / conflicts
      ↓
plan frontier
      ↓
observe
      ↓
interpret
      ↓
resolve identities
      ↓
validate candidates
      ↓
          ┌───────────────┐
          │               │
        fits          does not fit
          │               │
          ▼               ▼
     integrate           park
          │               │
          │           accumulate
          │               │
          │         infer structure
          │               │
          │       propose schema change
          │               │
          └───────◄───────┘
                  │
                  ▼
              maintain
                  │
                  ▼
             next revision
                  │
                  ↺
```

A simplified Rust sketch:

```rust
loop {
    let snapshot = runtime.snapshot();

    let frontier = planner.plan(&snapshot)?;

    let observations =
        crawler.crawl(&CrawlContext {
            graph: &snapshot,
            frontier: &frontier,
            budget,
        }).await?;

    let interpretations =
        interpreter.interpret(&snapshot, observations).await?;

    let resolved =
        resolver.resolve(&snapshot, interpretations).await?;

    let plans =
        integrator.plan(&snapshot, resolved).await?;

    let validated =
        validators.validate(&snapshot, plans)?;

    runtime.commit(validated)?;

    runtime.maintain()?;
}
```

---

# 34. Revision Model

Each successful commit produces a new immutable root.

```rust
pub struct Root {
    pub revision: u64,
    pub parent: Option<RootHash>,

    pub ontology_root: Hash,
    pub knowledge_root: Hash,
    pub evidence_root: Hash,
    pub agent_root: Hash,

    pub transaction: TransactionHash,
}
```

Revision lineage:

```text
Seed
 ↓
Root₀
 ↓
Root₁
 ↓
Root₂
 ↓
Root₃
```

This provides:

- reproducibility,
- auditability,
- rollback analysis,
- deterministic replay,
- schema migration traceability.

---

# 35. Removal and Forgetting

A system that only appends knowledge will eventually become unusable.

Forgetting must be a first-class subsystem.

There are three distinct forms.

## 35.1 Semantic removal

The active system should no longer treat a fact as true.

Usually represented by:

- retraction,
- supersession,
- end of valid-time interval.

## 35.2 Retention removal

Intermediate data is no longer useful enough to retain.

Examples:

- failed interpretations,
- stale transient roots,
- temporary model outputs,
- obsolete extraction structures.

## 35.3 Physical reclamation

Persisted bytes are actually deleted from storage.

This occurs only after reachability, policy, retention, and audit requirements permit it.

---

# 36. Canonical Retraction

Canonical knowledge is usually not physically erased when it ceases to be active.

```rust
pub enum AssertionStatus {
    Active,

    Retracted {
        at_revision: u64,
        reason: RetractionReason,
    },

    Superseded {
        by: AssertionId,
    },
}
```

Historical knowledge remains reconstructable.

Current queries can expose only active assertions by default.

---

# 37. Storage Classes

Different information deserves different durability.

```rust
pub enum StorageClass {
    Canonical,
    Provenance,
    Incubating,
    Cache,
    Ephemeral,
}
```

Suggested semantics:

## Canonical

- durable,
- revisioned,
- strongly governed,
- semantically retracted rather than casually deleted.

## Provenance

- retained while required to verify, audit, or reproduce canonical assertions.

## Incubating

- retained while it has meaningful integration or schema-discovery potential.

## Cache

- reproducible and freely deletable.

## Ephemeral

- short-lived agent and runtime working state.

---

# 38. Garbage Collection

Physical reclamation should be based partly on reachability.

Root set examples:

```text
current canonical root
audit holds
legal holds
pinned evidence
active transient roots
schema migration dependencies
explicitly retained historical roots
```

The collector traces all required references.

Everything unreachable becomes a reclamation candidate.

Conceptually:

```rust
fn collect(
    roots: &[ObjectId],
    store: &Store,
) -> GarbageSet {
    let reachable = trace_references(roots, store);

    store
        .all_objects()
        .filter(|id| !reachable.contains(id))
        .collect()
}
```

Unreachable does not necessarily mean immediately deletable.

---

# 39. Reclamation Grace Period

Physical deletion should normally pass through:

```text
unreachable
   ↓
candidate
   ↓
quarantine
   ↓
grace period
   ↓
physical deletion
```

This protects against:

- delayed references,
- bugs,
- mistaken schema migrations,
- accidental policy changes,
- temporary disconnection.

---

# 40. Decay of Incubating Knowledge

The Incubation Forest needs explicit decay pressure.

Otherwise it will become the dominant storage cost.

Possible root utility metadata:

```rust
pub struct RootUtility {
    pub last_accessed: Timestamp,
    pub last_changed: Timestamp,
    pub integration_progress: f32,
    pub information_gain: f32,
    pub canonical_references: u32,
    pub unresolved_conflicts: u32,
    pub schema_signal_strength: f32,
    pub storage_cost: u64,
}
```

The maintenance loop can ask:

- has this root produced useful canonical knowledge?
- has it revealed schema structure?
- is it still changing?
- does another root subsume it?
- is unique evidence stored here?
- is anyone depending on it?
- is expected future integration value above cost?

Low-value roots may be archived, compressed, or deleted.

---

# 41. Consolidation

The system should be able to forget representations without forgetting knowledge.

Consider:

```text
10,000 Slack messages
2,800 transient entities
430 candidate assertions
73 canonical assertions
```

After integration, the runtime may no longer need:

- every prompt,
- every extraction variant,
- all failed parse structures,
- redundant embeddings,
- duplicate candidate entities,
- obsolete transient edges.

It may retain:

- 73 canonical assertions,
- minimal required provenance,
- content hashes,
- selected source fragments,
- unresolved residual knowledge.

---

# 42. Semantic Compression

Some transient structures may be too valuable to delete but too expensive to retain in full.

They can be summarized into a smaller graph.

Example:

```text
50,000 messages
       ↓
semantic consolidation
       ↓
Project Atlas
 ├── recurring topic: migration
 ├── unresolved decision: provider choice
 ├── inferred vocabulary:
 │      amber/green/red = project health
 └── selected evidence references
```

The bulk interpretation graph can then be reclaimed according to policy.

---

# 43. Maintenance Loop

The complete runtime loop includes both epistemic growth and decay.

```text
observe
   ↓
interpret
   ↓
integrate / park
   ↓
reconcile
   ↓
schema-discover
   ↓
compress
   ↓
expire
   ↓
archive
   ↓
garbage collect
   ↓
repeat
```

Growth without maintenance is considered an invalid long-term operating mode.

---

# 44. Contradictions

Contradictory assertions should not automatically destroy one another.

The system may represent competing claims explicitly.

Example:

```text
Assertion A:
    ProjectAtlas.launch_date = Oct 12
    evidence = source X

Assertion B:
    ProjectAtlas.launch_date = Oct 19
    evidence = source Y
```

The contradiction subsystem can:

- mark assertions disputed,
- assess source freshness,
- inspect temporal context,
- seek additional evidence,
- identify supersession,
- escalate to human review.

Contradiction is a state to investigate, not merely an error to suppress.

---

# 45. Entity Resolution

Entity resolution is one of the highest-risk semantic operations.

Transient interpretation may create:

```text
SarahCandidate#1
SarahCandidate#2
"Sarah"
```

The runtime must determine whether these correspond to:

```text
Canonical Person: Sarah Chen
```

Possible signals:

- source identity,
- explicit IDs,
- email or account references,
- contextual relationships,
- temporal overlap,
- matching attributes,
- human confirmation.

Incorrect merges can corrupt large portions of a knowledge graph.

Therefore merges should be explicit transactions with provenance.

---

# 46. Merge and Split Semantics

Entities sometimes need to merge or split.

## Merge

```text
candidate A
candidate B
   ↓
same canonical entity
```

The runtime should preserve alias and historical lineage rather than rewriting identity blindly.

## Split

A previously merged entity may later be discovered to represent multiple real entities.

The runtime needs a migration transaction that:

- creates distinct entities,
- rewrites affected references,
- preserves historical provenance,
- flags uncertain assignments.

---

# 47. Query Semantics

Queries should be explicit about epistemic scope.

Examples:

```text
canonical only
canonical + disputed
canonical as of revision N
canonical valid at time T
include incubating roots
source-specific interpretation
all claims regardless of acceptance
```

The default application query should generally use canonical active knowledge.

Research or debugging tools may expose broader scopes.

---

# 48. Trust and Agent Reliability

Agent reliability can be tracked but should never substitute for evidence.

```rust
pub struct TrustProfile {
    pub evidence_extraction: f32,
    pub entity_resolution: f32,
    pub ontology_reasoning: f32,
}
```

Trust profiles can influence:

- review intensity,
- scheduling,
- fallback requirements,
- whether another validator is required.

They should not independently convert assertions into truth.

---

# 49. Consensus

Some changes may require multiple independent validations.

For example:

```text
Extractor Agent A
      ↓
Candidate
      ↓
Reference Validator B
      ↓
Evidence Validator C
      ↓
Ontology Validator D
      ↓
Commit Gate
```

Independence can be logical rather than necessarily model-level.

For high-impact changes, different models or humans may be required.

---

# 50. Schema Governance

Not all schema changes are equal.

Possible risk classes:

## Low risk

- new optional property,
- new subtype with no migration,
- local alias mapping.

## Medium risk

- new relation semantics,
- cardinality expansion,
- non-breaking constraint change.

## High risk

- changing identity rules,
- narrowing cardinality,
- changing property type,
- modifying canonical dependency semantics,
- altering validation requirements.

Higher-risk classes require stronger approval policies.

---

# 51. Meta-Ontology

Eventually the ontology itself may be represented using graph primitives.

Example:

```text
Type
 ├── has_property → Property
 ├── subtype_of → Type
 └── constrained_by → Constraint

Property
 ├── range → Type
 ├── cardinality → Cardinality
 └── value_kind → ValueKind
```

This creates:

```text
Meta-schema
    ↓ describes
Ontology
    ↓ types
Knowledge Graph
```

The meta-schema belongs close to the trusted kernel and should evolve cautiously.

---

# 52. Schema Compatibility

Transient schemas do not need to equal canonical schemas.

The system should support explicit schema compatibility mappings.

Examples:

```text
Slack.Thread → Canonical.Conversation
Slack.TentativeAction → maybe Canonical.Task
Document.PersonMention → Canonical.Person reference
```

Mappings may be:

- exact,
- lossy,
- conditional,
- one-to-many,
- many-to-one,
- non-integratable.

Mappings themselves should be versioned.

---

# 53. Local Stability vs Canonical Acceptance

A transient schema can be stable without becoming canonical.

This distinction matters.

Example:

```text
Slack Interpretation Schema v4
```

may remain useful indefinitely for interpreting Slack observations.

It need not be merged into the universal ontology.

Canonical ontology should avoid becoming a dumping ground for source-specific implementation details.

---

# 54. Source Adapters

Source ingestion should be modular.

Possible adapter interface:

```rust
pub trait SourceAdapter {
    async fn poll(
        &self,
        checkpoint: SourceCheckpoint,
    ) -> Result<ObservationBatch, SourceError>;
}
```

Examples:

- Slack adapter,
- Gmail adapter,
- Git adapter,
- filesystem adapter,
- HTTP crawler,
- database CDC adapter.

Each adapter should preserve source-native identifiers when available.

---

# 55. Checkpoints and Incremental Ingestion

The system should favor deltas over full re-ingestion.

Examples:

```text
Slack:
    messages after timestamp X

Git:
    commits after hash H

Database:
    CDC offset O

Document:
    content hash changed
```

Checkpoint state should be durable but not confused with canonical knowledge.

---

# 56. Idempotency

Repeated ingestion must not create duplicate semantic objects merely because the same source was observed twice.

Useful mechanisms include:

- source-native IDs,
- content hashes,
- transaction IDs,
- deterministic observation IDs,
- deduplication indexes.

Idempotency should exist at both the observation and integration layers.

---

# 57. Content Addressing

Raw source payloads and immutable artifacts benefit from content addressing.

Example:

```rust
pub struct ContentHash([u8; 32]);
```

Advantages:

- deduplication,
- integrity verification,
- reproducible provenance,
- immutable evidence references,
- efficient storage.

---

# 58. Security and Authorization

The runtime may ingest knowledge from sources with different access controls.

Canonical integration must not erase source authorization semantics.

Questions include:

- who may read an assertion?
- who may read its provenance?
- may derived knowledge be exposed more broadly than the source?
- what happens when source access is revoked?

Authorization should therefore be attached to:

- observations,
- evidence,
- transient roots,
- canonical assertions where necessary.

---

# 59. Derived Knowledge and Information Leakage

A canonical assertion derived from restricted source material may itself reveal restricted information.

Therefore integration must perform an access-policy derivation step.

A safe rule is not necessarily:

```text
canonical = universally visible
```

Instead:

```text
canonical = epistemically accepted
```

while access control remains orthogonal.

---

# 60. Deletion Requests and Data Governance

Physical deletion may be required independently of epistemic history.

The runtime therefore needs policy-aware deletion.

Examples:

- user deletion request,
- source revocation,
- contractual retention limits,
- legal requirements,
- privacy policy.

When evidence must be deleted, dependent canonical assertions may need to:

- retain only an allowed derived form,
- lose provenance status,
- become disputed,
- be revalidated from alternate evidence,
- be retracted.

---

# 61. Observability

The runtime should expose metrics for epistemic health.

Examples:

## Canonical metrics

- total active assertions,
- unresolved contradictions,
- assertions without sufficient provenance,
- schema validation failures,
- stale assertions,
- retractions per period.

## Incubation metrics

- number of transient roots,
- storage size,
- average root age,
- integration rate,
- abandoned root rate,
- emergent schema proposal count.

## Agent metrics

- proposal acceptance rate,
- false merge corrections,
- validation disagreement rate,
- extraction failure rate.

## Maintenance metrics

- reclaimed bytes,
- consolidation ratio,
- expired observations,
- unreachable object count.

---

# 62. Explainability

Every canonical assertion should be explainable through a chain such as:

```text
Why is this true?

Canonical Assertion
    ↓
Accepted Transaction
    ↓
Validation Results
    ↓
Integration Plan
    ↓
Transient Interpretation
    ↓
Evidence
    ↓
Observation
    ↓
External Source
```

The exact chain may vary, but provenance should never end at "the model inferred it" for high-trust knowledge.

---

# 63. Example: Slack-to-Canonical Flow

Input message:

```text
"Sarah: Let's move the Atlas deployment to Tuesday."
```

## Step 1: Observation

```text
Observation O991
source = Slack channel X
message_id = M123
timestamp = ...
content_hash = ...
```

## Step 2: Interpretation

```text
TransientRoot T201

Message M123
    speaker → SarahCandidate
    refers_to → AtlasCandidate
    proposes → RescheduleCandidate
    target_date → Tuesday
```

## Step 3: Resolution

```text
SarahCandidate
    → Canonical Person Sarah

AtlasCandidate
    → Canonical Project Atlas
```

## Step 4: Semantic decision

The system determines that the sentence expresses a proposal, not necessarily a completed schedule change.

It may therefore integrate:

```text
Sarah
    proposed_reschedule
        → AtlasDeployment
```

rather than:

```text
AtlasDeployment.date = Tuesday
```

unless additional evidence indicates a committed decision.

## Step 5: Provenance

The canonical assertion retains provenance to `O991`.

## Step 6: Later message

```text
"Confirmed, Atlas deploy is Tuesday."
```

A later observation may allow:

```text
AtlasDeployment.date = Tuesday
```

to cross the canonical gate.

---

# 64. Example: Ontology Discovery

Repeated transient facts:

```text
Atlas.health = "amber"
Nova.health = "green"
Orion.health = "red"
```

Canonical ontology lacks `ProjectHealth`.

The schema-discovery loop detects a recurring pattern.

Proposal:

```text
Type: ProjectHealth
Enum: Green | Amber | Red
Property: Project.health -> ProjectHealth
```

After schema approval:

```text
Schema v17 → Schema v18
```

The runtime revisits parked roots.

Previously incompatible facts can now be integrated.

---

# 65. Example: Retraction

Canonical state:

```text
Alice CEO_OF Acme
```

New evidence shows:

```text
Bob became CEO on 2026-03-12
```

The system does not delete Alice's historical relationship.

Instead:

```text
Alice CEO_OF Acme
valid_to = 2026-03-12

Bob CEO_OF Acme
valid_from = 2026-03-12
```

The current-world query returns Bob.

Historical query remains possible.

---

# 66. Example: Transient Root Expiry

A temporary source dump produces:

```text
TransientRoot T982
age: 120 days
integration_progress: 0
schema_signal_strength: low
unique evidence: none
canonical dependencies: none
last_accessed: 90 days ago
```

Maintenance policy may:

1. mark root as expired,
2. place it in quarantine,
3. retain metadata and hashes briefly,
4. physically reclaim underlying graph objects.

---

# 67. Example: Consolidation

A large document collection produces 80,000 transient nodes.

After integration:

```text
425 canonical assertions
38 unresolved claims
6 schema signals
```

Consolidation may preserve:

- the 425 canonical assertions,
- their required evidence,
- the 38 unresolved claims,
- the 6 schema signals,
- selected representative examples.

Most intermediate graph structure can be reclaimed.

---

# 68. Rust Domain Sketch

A conceptual module layout:

```text
epistemic_runtime/
├── kernel/
│   ├── identity.rs
│   ├── references.rs
│   ├── transactions.rs
│   └── invariants.rs
│
├── ontology/
│   ├── types.rs
│   ├── properties.rs
│   ├── edges.rs
│   ├── constraints.rs
│   └── migration.rs
│
├── graph/
│   ├── canonical.rs
│   ├── transient.rs
│   ├── snapshot.rs
│   ├── revision.rs
│   └── root.rs
│
├── assertion/
│   ├── assertion.rs
│   ├── temporal.rs
│   ├── provenance.rs
│   └── validation_state.rs
│
├── observation/
│   ├── observation.rs
│   ├── evidence.rs
│   └── source.rs
│
├── agents/
│   ├── agent.rs
│   ├── capabilities.rs
│   ├── extraction.rs
│   ├── resolution.rs
│   ├── validation.rs
│   └── schema_discovery.rs
│
├── integration/
│   ├── plan.rs
│   ├── mapping.rs
│   ├── promotion.rs
│   └── reconciliation.rs
│
├── crawler/
│   ├── crawler.rs
│   ├── frontier.rs
│   ├── planner.rs
│   └── budget.rs
│
├── maintenance/
│   ├── retention.rs
│   ├── consolidation.rs
│   ├── expiry.rs
│   └── gc.rs
│
└── runtime/
    ├── loop.rs
    ├── scheduler.rs
    ├── store.rs
    └── metrics.rs
```

---

# 69. Suggested Rust Core Types

```rust
pub struct RuntimeState {
    pub canonical: CanonicalGraph,
    pub incubation: IncubationForest,
    pub observations: ObservationStore,
    pub evidence: EvidenceStore,
    pub schemas: SchemaRegistry,
    pub revision: u64,
}
```

```rust
pub struct IncubationForest {
    pub roots: BTreeMap<TransientRootId, TransientGraph>,
}
```

```rust
pub struct GraphRoot {
    pub id: GraphRootId,
    pub schema: SchemaVersionId,
    pub parent: Option<GraphRootId>,
    pub created_at: Timestamp,
    pub lifecycle: KnowledgeState,
}
```

```rust
pub struct SchemaRegistry {
    pub canonical: SchemaVersionId,
    pub schemas: BTreeMap<SchemaVersionId, Ontology>,
}
```

---

# 70. Runtime API Boundary

The runtime should expose a small controlled interface.

```rust
pub trait KnowledgeRuntime {
    fn snapshot(&self) -> RuntimeSnapshot;

    fn ingest(
        &mut self,
        observations: Vec<Observation>,
    ) -> Result<IngestResult, RuntimeError>;

    fn propose(
        &self,
        proposal: GraphTransaction,
    ) -> Result<ProposalId, RuntimeError>;

    fn validate(
        &self,
        proposal: ProposalId,
    ) -> Result<ValidatedTransaction, ValidationReport>;

    fn commit(
        &mut self,
        tx: ValidatedTransaction,
    ) -> Result<CommitResult, CommitError>;

    fn maintain(
        &mut self,
    ) -> Result<MaintenanceReport, RuntimeError>;
}
```

Agents operate through this boundary.

---

# 71. Snapshot Semantics

Readers should normally operate on immutable snapshots.

```rust
pub struct GraphSnapshot<'a> {
    graph: &'a CanonicalGraph,
    revision: u64,
}
```

Advantages:

- consistent reads,
- deterministic validation,
- clear conflict detection,
- concurrency safety,
- reproducibility.

A validated transaction records which revision it was validated against.

If canonical state changes before commit, revalidation may be required.

---

# 72. Concurrency

Many agents may operate simultaneously.

The runtime should therefore support optimistic concurrency.

Typical flow:

```text
read snapshot R100
   ↓
build proposal
   ↓
validate against R100
   ↓
canonical becomes R101
   ↓
commit attempt notices mismatch
   ↓
rebase / revalidate
```

Canonical commits remain serialized or otherwise globally ordered.

---

# 73. Deterministic vs Agentic Components

A useful design principle is:

> Use agents where interpretation is necessary; use deterministic code where invariants can be expressed precisely.

Deterministic:

- identity format,
- reference existence,
- type checking,
- cardinality,
- transaction integrity,
- revision lineage,
- dependency boundaries,
- retention rules.

Agentic:

- entity resolution under ambiguity,
- semantic extraction,
- contradiction interpretation,
- ontology proposal discovery,
- integration mapping discovery,
- prioritization under uncertainty.

---

# 74. Failure Modes

## 74.1 Ontology overgrowth

Every new phrase becomes a new type.

Mitigation:

- strong schema governance,
- pattern thresholds,
- preference for mappings,
- ontology simplicity metrics.

## 74.2 Canonical contamination

Weak transient interpretations leak into trusted state.

Mitigation:

- hard dependency boundary,
- explicit integration plans,
- typed canonical references,
- validation gates.

## 74.3 Permanent incubation landfill

Parked data grows forever.

Mitigation:

- decay scoring,
- expiry,
- consolidation,
- GC,
- storage budgets.

## 74.4 Entity merge corruption

Two entities are incorrectly unified.

Mitigation:

- explicit merge transactions,
- reversible lineage,
- merge confidence thresholds,
- stronger validation for high-degree entities.

## 74.5 Circular provenance

Assertion A supports B while B supports A.

Mitigation:

- provenance DAG rules,
- evidence-class distinction,
- cycle detection.

## 74.6 Schema migration breakage

A new schema invalidates existing canonical knowledge.

Mitigation:

- migration simulation,
- compatibility validation,
- revision snapshots,
- atomic schema commits.

## 74.7 Agent collusion by shared error

Multiple validators reproduce the same model error.

Mitigation:

- deterministic validators where possible,
- heterogeneous validators,
- source verification,
- independence requirements for critical claims.

---

# 75. Epistemic Health

The system should measure not only quantity of knowledge but quality.

Possible indicators:

```text
coverage
provenance completeness
contradiction density
staleness
schema stability
integration latency
transient-to-canonical conversion rate
retraction frequency
unresolved identity ambiguity
maintenance cost
```

The objective should not be maximal graph size.

The objective is useful, dependable, maintainable knowledge.

---

# 76. Design Philosophy

Several principles guide the system.

## 76.1 Preserve ambiguity until justified

Do not force uncertain knowledge into overly precise canonical structures.

## 76.2 Make trust boundaries explicit

Do not let UI labels or confidence scores substitute for architectural isolation.

## 76.3 Prefer immutable derivation

Generate new revisions rather than mutating historical meaning in place.

## 76.4 Integrate knowledge, not raw representations

Canonical state should capture useful semantics, not mirror every source format.

## 76.5 Let ontology evolve slowly

Knowledge can arrive quickly.

Schema should evolve more cautiously.

## 76.6 Treat forgetting as healthy

Deletion, decay, and consolidation are necessary system behaviors.

## 76.7 Preserve evidence where it matters

The system should always be able to answer "why do we believe this?" for high-value canonical knowledge.

---

# 77. Conceptual Identity of the System

The most precise conceptual description is:

> A self-maintaining epistemic knowledge runtime that continuously transforms observations into typed, provenance-aware, validated, revisable knowledge.

A more architectural description is:

> A Knowledge Fabric maintained by an Epistemic Loop, with a strongly validated Canonical Core surrounded by an Incubation Forest of transient knowledge.

A concise product-oriented description is:

> A self-evolving knowledge runtime.

---

# 78. Naming Vocabulary

Recommended internal vocabulary:

| Concept | Preferred Name |
|---|---|
| whole system | Epistemic Knowledge Runtime |
| all stored knowledge spaces | Knowledge Fabric |
| highest-integrity graph | Canonical Core |
| non-canonical graph roots | Incubation Forest |
| incoming source records | Observation Layer |
| semantic acquisition planner | Knowledge Frontier |
| continuous state process | Epistemic Loop |
| promotion into canonical state | Integration |
| removal of redundant representation | Consolidation |
| invalidating active belief | Retraction |
| physical deletion | Reclamation |
| schema/type system | Ontology |
| immutable system history | Revision Lineage |

---

# 79. Minimal Operating Loop

At its simplest, the architecture can be reduced to:

```text
SEED
  ↓
observe
  ↓
interpret
  ↓
validate
  ↓
integrate or park
  ↓
reconcile
  ↓
evolve schema if justified
  ↓
compress / expire / collect
  ↓
new state
  ↺
```

The system should remain conceptually understandable in this form even if the implementation becomes distributed and complex.

---

# 80. Final Architectural Principle

The defining feature of this system is not that AI agents operate on a graph.

It is that the runtime formalizes the lifecycle of knowledge:

```text
see
 ↓
interpret
 ↓
believe tentatively
 ↓
cross-check
 ↓
integrate
 ↓
depend upon
 ↓
reconsider
 ↓
retract or reinforce
 ↓
compress
 ↓
forget
 ↓
observe again
```

The graph is the memory substrate.

The ontology is the type system.

The agents are interpreters and workers.

The validators form the integrity membrane.

The incubation forest is working memory.

The canonical core is dependable long-term knowledge.

The epistemic loop is the organism-like process that continuously connects them.

That loop—not any individual graph representation—is the central object of the architecture.

---

# Amendments — 2026-09-21

Sections 1–80 are the original design. The sections below add what the two predecessor systems,
`company-brain` (v1) and `org-brain` (v2), proved necessary and the original does not say. The
survey behind them, with the paths and counts, is `docs/predecessors.md`; the ids A1–A15 there
name the capabilities each amendment carries. An amendment adds; it does not rewrite. Where one
extends an earlier section it says so.

---

# 81. Operator Surface

*Carries A1. Extends § 18 and § 44.*

The runtime has a person in its loop. § 18 lists human reviewers among the agent implementations
and § 44 escalates contradictions to human review, but neither says how a question reaches a person
or how an answer returns. Both predecessors converged on the same protocol: a rendered page of
numbered items, and an answer addressed by number.

## 81.1 Attention items

An attention item is something the runtime cannot resolve without a human signal:

```rust
pub struct AttentionItem {
    pub id: AttentionId,
    pub number: String,           // stable within one rendered page, e.g. "5c"
    pub kind: AttentionKind,
    pub subject: Subject,
    pub options: Vec<AttentionOption>,
    pub rendered_at_revision: u64,
}

pub enum AttentionKind {
    Disputed { assertions: Vec<AssertionId> },
    UnresolvedIdentity { candidates: Vec<NodeId> },
    SchemaProposal { proposal: SchemaProposalId, risk: RiskClass },
    ObligationDue { obligation: NodeId },
    IntegrationBlocked { root: GraphRootId, reason: IntegrationBlock },
    ApprovalRequested { write: OutwardWriteId },
}
```

The queue is a projection over a snapshot (§ 82). It is not a store of its own: an item exists
because the canonical or incubating state has the shape that produces it, and it disappears when
that shape changes.

## 81.2 Answers

An answer is an observation, not a privileged write:

```rust
pub struct HumanStatement {
    pub identity: AgentId,        // an authenticated person, never a free-text name
    pub answers: AttentionId,
    pub seen_at_revision: u64,
    pub statement: Statement,     // ChooseOption | Assert(Value) | Reject { reason } | Defer { until }
}
```

The statement becomes `EvidenceSource::HumanStatement` (§ 16), and the operator-surface agent
proposes the transaction it implies. That transaction passes the validation pipeline like any other.
What the human signal changes is provenance: policy may state that a `HumanStatement` from an
authenticated identity is sufficient provenance for a class of assertions, where an agent's
inference is not (§ 6.5).

An answer records the revision it was given against. If canonical state moved in between, the
proposal is revalidated (§ 72), and the person is not asked the same question twice for the same
subject unless the evidence changed.

---

# 82. Projections and Views

*Carries A2. Extends § 47.*

The original stops at query scopes. Both predecessors were read through rendered pages: an
attention page, a personal brief, a digest, an explain page. These are projections.

```rust
pub struct View {
    pub template: TemplateId,       // versioned
    pub scope: QueryScope,          // § 47
    pub revision: u64,
    pub content_hash: ContentHash,
}
```

Rules:

- A view is a deterministic function of a snapshot at a revision, a template version and a query
  scope. Two renders of the same triple are byte-identical.
- Views are `StorageClass::Cache` (§ 37): reproducible and freely deletable.
- Every fact a view renders carries the id of the assertion it came from, so a reader can ask
  "why is this here?" and reach § 62.
- A fact past its horizon (§ 84) renders as unknown, never as false and never silently as current.
- Delivering a view anywhere outside the runtime — posting it to a channel, sending it as a message
  — is an outward write under § 83. Rendering is not delivery.

---

# 83. Invariant 6.11 — Outward Writes

*Carries A3. Adds an invariant to § 6.*

> **6.11 The runtime reads the world. Every outward write is an approved transaction.**

An outward write is any effect beyond the runtime's own stores: a message sent, a ticket changed, a
page published, a call to any API that is not read-only. Both predecessors held this rule — v1
never sent a draft automatically; v2 refused `communication.send` without an approval record — and
both treated it as the boundary that made the rest safe to run unattended.

```rust
pub struct OutwardWrite {
    pub id: OutwardWriteId,
    pub target: EffectTarget,
    pub payload_hash: ContentHash,
    pub approval: ApprovalRef,
}

pub enum ApprovalRef {
    Statement(EvidenceId),         // a HumanStatement of kind Approve for this write
    StandingPolicy(NodeId),        // a canonical node granting a scoped standing approval
}
```

Consequences:

- `SourceAdapter` (§ 54) is read-only by construction; its trait exposes `poll` and nothing that
  writes. Effects go through a separate `EffectAdapter` that accepts only a `ValidatedTransaction`
  containing an `OutwardWrite` whose `approval` resolved during validation.
- The authorization validator (§ 20, item 7) refuses an `OutwardWrite` whose approval does not
  resolve, is out of scope, or has expired.
- A draft is knowledge: a `Draft` node whose content is never delivered until an `OutwardWrite`
  cites it with an approval.
- A standing policy is canonical knowledge with a scope (target, kind, until) and is itself
  proposed, validated and revisable.

---

# 84. Obligations and Horizons

*Carries A4. Extends § 14 and § 31.*

Bitemporal validity says when a fact was true and when the runtime believed it. It does not say
that something is owed, or that a belief has gone stale. Both predecessors carried both.

## 84.1 Obligations

The seed ontology includes an obligation type:

```rust
Obligation
    owner:   NodeRef(Person | Agent)
    subject: NodeRef(any)
    due:     Timestamp | Recurrence
    state:   Enum { open, met, missed, waived }
```

Commitments, replies owed, deadlines and recurring duties are obligations. The runtime's clock is
an input to the loop, never read inside the kernel; comparing `due` with the supplied `now` yields
the frontier task:

```rust
KnowledgeTask::ObligationDue { obligation: NodeId }
```

which surfaces as `AttentionKind::ObligationDue` (§ 81) when the owner is a person.

## 84.2 Horizons

Every assertion derived from an external source carries a horizon:

```rust
pub struct Horizon {
    pub observed_at: Timestamp,
    pub valid_for: Duration,      // from the source's policy
}
```

Past `observed_at + valid_for` without re-observation, the assertion is stale: queries mark it,
views render it as unknown (`?`), and the frontier emits `RefreshStaleKnowledge` (§ 31). A stale
assertion is neither retracted nor false; it is a belief whose currency the runtime can no longer
vouch for. This is distinct from valid time: the world may not have changed, but the runtime has
not looked.

---

# 85. Interpretation Session Contract

*Carries A5. Extends § 18 and § 24.*

Agents are abstract in the original. The predecessor that ran a paid model unattended (v2) arrived
at a contract that makes a model session auditable and its output admissible, and the shape is the
same for any interpreter, model or not.

```rust
pub struct InterpretationRequest {
    pub id: RequestId,
    pub context_digest: ContentHash,          // over every byte the interpreter sees
    pub signals: Vec<ObservationId>,          // the exact input set
    pub held: Vec<HeldSubject>,               // current canonical subjects the interpreter may compare against, with revisions
    pub schema: SchemaVersionId,
    pub byte_bound: u64,
    pub effort: Effort,
}

pub struct InterpretationResult {
    pub request: ContentHash,                 // digest of the request it answers
    pub dispositions: Vec<Disposition>,       // exactly one per input signal, ids copied verbatim
}

pub enum Disposition {
    Propose { signal: ObservationId, tx: GraphTransaction },
    NoOp    { signal: ObservationId, basis: NoOpBasis },
}

pub enum NoOpBasis {
    Noise,
    InsufficientEvidence,
    Privacy,
    AlreadyHeld        { comparisons: Vec<HeldComparison> },
    ComparisonWithheld { comparisons: Vec<HeldComparison> },
}

pub struct Receipt {
    pub request: ContentHash,
    pub result: ContentHash,
    pub completed_at: Timestamp,
    pub cost: Cost,
}
```

Rules, all deterministic and checked before any proposal enters § 20:

- The request is immutable and content-addressed; the interpreter sees the request and nothing
  else. Source material inside it is data, never instruction.
- The result names every input signal exactly once, by the request's own identifier. A missing,
  extra or renamed identity refuses the whole result.
- A no-op carries a basis. `AlreadyHeld` and `ComparisonWithheld` cite exact held subjects and
  revisions from the request; a comparison against a subject created in the same result refuses.
- A receipt binds request digest, result digest, completion and cost. No receipt, no admission.
- A request has a bounded number of visits. A failed session has no edge back to interpretation
  except through an explicit, recorded recovery.
- Sensitive held fields are withheld from the request and named in `ComparisonWithheld`; credential
  redaction runs before the request is sealed (predecessors A6).

Cost is recorded per receipt and aggregated by the scheduler (§ 32 gains a real ledger in the
roadmap's P5).

---

# 86. Blob Evidence

*Carries A12. Extends § 15.*

`ObservationContent` gains a variant for bytes the runtime stores but does not parse:

```rust
ObservationContent::Blob {
    hash: ContentHash,
    media_type: String,
    byte_len: u64,
}
```

Bytes live in the content-addressed store under `StorageClass::Provenance` while any canonical
assertion cites them, else `Cache`. Extraction from a blob — text from a PDF, a transcript from
audio — is an interpretation (§ 85) whose evidence is the blob's hash and the extractor's identity.
Blobs are subject to § 60 deletion like any evidence.

---

# 87. Per-Type Lifecycles and Operations

*Carries A13. Extends § 11 and § 19.*

The original ontology gives a node type properties and constraints. v2 proved that the form people
review and the form that refuses a wrong move is a type with a lifecycle and named operations: a
decision that is `open` may become `decided`; a `decided` decision may not become `moot` through a
generic property update.

```rust
pub struct NodeType {
    // ... § 11.1
    pub lifecycle: Option<Lifecycle>,
    pub operations: BTreeMap<String, OperationDefinition>,
}

pub struct Lifecycle {
    pub initial: String,
    pub states: BTreeSet<String>,
    pub transitions: BTreeSet<(String, String)>,
}

pub struct OperationDefinition {
    pub arguments: BTreeMap<String, ValueType>,
    pub preconditions: Vec<Constraint>,
    pub transition: Option<(String, String)>,
    pub sets: BTreeMap<PropertyId, ValueTemplate>,
    pub emits: Vec<EventType>,
}
```

`GraphOperation` (§ 19) gains:

```rust
GraphOperation::Invoke {
    node: NodeId,
    operation: String,
    arguments: BTreeMap<String, Value>,
}
```

The ontology-constraint validator (§ 20, item 5) refuses an `Invoke` whose precondition fails or
whose transition is not declared. A node's lifecycle state is a typed property `state: Enum` over
the lifecycle's states.

This is the world's lifecycle — whether a decision is decided, whether a person has departed. It is
orthogonal to `KnowledgeState` (§ 29), which is about knowing — whether the runtime has integrated
the node. One node carries both.

Lifecycles and operations are part of the schema and evolve through § 26 schema transactions at the
risk class § 50 assigns: a new optional operation is low risk; removing a transition existing nodes
have taken is high risk.

# 88. Assertion Assessment, Lifecycle and Historical Reads

*Dated 2026-09-22. Resolves the overlap between §§17 and 36, and makes §65's two time dimensions explicit.
Implementation is unexecuted at this amendment's preparation.*

An assertion stores its validation assessment and lifecycle independently. Validation retains
Proposed, Validating, Accepted, Rejected and Disputed, with their existing payloads. Accepted
retains the exact accepting validator set. Lifecycle is stored, not derived from validation:

- Active: no withdrawal or replacement is recorded.
- Retracted: the committing revision and stated reason.
- Superseded: the replacing assertion, committing revision and effective valid-time boundary.

Lifecycle changes preserve the assessment, evidence identities and proposer. Neither Active nor
Superseded grants acceptance. A fresh proposal supplies Proposed assessment and Active lifecycle;
only kernel validation establishes acceptance, and only a validated lifecycle operation changes
lifecycle. The general assessment policy remains distinct from this storage shape.

A retraction says the runtime no longer endorses the claim. At that revision and later,
valid-time reads exclude it at every time. A supersession records a supported fact's replacement:
the earlier claim remains answerable within its closed valid-time interval, provided its retained
assessment is Accepted. The replacement must itself be Accepted in the validated candidate,
have a distinct assertion identity, begin at the supplied boundary, and satisfy the candidate
valid_at eligibility predicate at that boundary. An already retracted or otherwise unreadable
replacement refuses even though it retains Accepted assessment. Subject and predicate
equality is not required: the §65 example replaces assertions about different subjects.
Normal ontology, provenance and cardinality checks still apply. The former interval
ends at that boundary; intervals are half-open. The boundary cannot precede the former interval's
start or extend a finite existing end. A self-reference or supersession cycle is refused.

For §65, a latest-revision query before the handover returns the earlier relationship; one at
or after the handover returns its replacement, subject to the replacement's own valid interval.
These are both valid-time reads of one revision. Reconstruction at an earlier committed revision
answers what that revision believed, before later retraction or supersession. Transaction-time
recording does not replace the effective valid time.

Ordinary transaction publication receives its timestamp from execution context and persists it
in the commit receipt. Newly accepted assertions have recorded_from set to that timestamp and
recorded_to unset. Retraction and supersession close the former record's recorded_to at that
timestamp, preserving recorded_from. A timestamp preceding the latest committed timestamp or
an affected record's recorded_from refuses; replay uses the recorded timestamp, never its clock.
Supersession's replacement receives the same recording timestamp when newly added; an already
accepted replacement keeps its recorded_from. Retraction leaves the historical valid interval
unchanged; supersession closes it at the supplied effective boundary.

At a selected revision, valid_at(t) includes exactly assertions with Accepted assessment, a
valid-time interval containing t, and either (Active lifecycle and open transaction time) or
Superseded lifecycle. Retracted assertions never qualify. Closing transaction time on a
Superseded record does not suppress its supported historical valid interval. Revision selection
supplies the transaction-history axis; valid_at does not guess a wall-clock instant.

AddAssertion and explicit supersession may occur in one unordered atomic transaction. Fresh
assertions arrive Proposed; assess replacement acceptance against the fully validated candidate,
not the pre-transaction graph. The replacement must survive and be readable at the boundary in
the candidate. Conflicting lifecycle
changes, retraction of that replacement, self-supersession and cycles refuse. These operations
are implemented by the writer; the format unit defines the retained state and read semantics.

This changes new-format assertion canonical encoding: assessment then lifecycle are both encoded
in declaration order, and either payload affects the hash. Preserve frozen old encoders for
legacy verification. Never reinterpret old Retracted or Superseded assessment variants by
inventing an accepting validator set. Prior acceptance must be recovered from independently
verifiable retained history or migration refuses with the affected assertion identity.

# 89. Revision Occurrences and Persisted Format Dispatch

*Dated 2026-09-22. Extends §§34, 56 and 57. Independent equal-content decisions are distinct facts; retrying a
decision does not produce another fact. Implementation is unexecuted at preparation.*

Every new revision-log fact is enclosed in a strict versioned record with an EventId:
format "ekr.revision-event/2", event_id, record_hash, payload. The mandatory record_hash
addresses the complete strict retained record for that event kind; duplicate identities,
counts and hashes in the event and retained record must agree. EventId is a UUID-backed stable identity,
allocated once by the kernel for the occurrence. The payload keeps the existing six event names
and variant indices. Canonical encoding includes format, event_id, record_hash and payload in that order.
Store the record with backend schema version 2; preserve backend event identity, stream position,
event name and schema version when reading it.

Object-event dispatch is independently explicit. The metadata-only ObjectStored body in §91.6
uses backend schema2. The original ObjectStored/schema1 body with inline bytes is frozen for
verification and preservation-first migration; it is not new-format ingestion after activation.
ObjectRetentionRaised retains its unchanged schema1 shape and meaning. Dispatch by the exact
event name and schema pair, never by the presence or absence of a bytes field. Unknown versions,
metadata-only schema1, inline schema2, unknown semantic fields and mismatched or missing required
blob bindings refuse. Preserve original schema1 bytes and hashes in the legacy verifier.

A new proposal, validation, refusal or stale decision receives a new occurrence identity even
when its payload repeats earlier content. A retry uses the same identity and exact payload.
Use occurrence identity for idempotency and complete record bytes for request equality. Reusing
an identity for different content refuses. Never generate a new identity inside a retrying append,
and never derive identity from content or the stream position about to be written.

Canonical validation addresses use ContentHash::of over the exact canonical transaction and
explicit validation basis, never ContentHash::of_bytes over encoded canonical values. Preserve the
existing payload/value hash labels. Version dispatch distinguishes the historical payload-domain
scheme from the corrected value-domain scheme; it never silently verifies an old hash with new
rules. The durable writer additionally binds acceptance to the full prior root and retained
validator policy; a revision number by itself does not prove equivalent state in different stores.
Section91 specifies the complete retained receipts and their exact basis before first publication.

New assertion shape requires explicit graph-document format dispatch. The compatibility table is:

| input | graph representation | admission |
|---|---|---|
| unversioned GraphDocument | original fields and assessment enum | legacy inventory and original hash verification only |
| ekr-seed/1 | original GraphDocument in graph | legacy inventory and verified migration only |
| ekr-seed-envelope/1 | input is ekr-seed/1; retained bootstrap context | legacy inventory and verified migration only |
| ekr.graph-document/2 | graph contains the new assertion assessment and lifecycle | new-format graph decoder |
| ekr-seed/2 | graph is an ekr.graph-document/2 envelope; ontology and evidence_payloads remain explicit | new kernel seed admission |
| ekr-seed-envelope/2 | input is ekr-seed/2; context, full authority state and committed_at are retained | new kernel seed replay admission |

The graph envelope is {format: "ekr.graph-document/2", graph: <graph fields>}. A seed's graph
field holds that complete envelope, not its inner graph. A persisted seed envelope holds the
complete versioned seed input in input. Every layer is strict. Unknown versions, mismatched
nesting, new graph shape under an old seed tag and unknown semantic fields refuse. Preserve
user-defined record keys as data. Successful conversion of a verifiable seed/1 is a migration
with an explicit address map, never normal ingestion under new defaults.

New-format node and edge properties store an ordered outer collection of values for each
PropertyId. A property's multiplicity counts that outer collection, preserving order and
duplicates. A single List value remains one outer member even when its inner list is empty.
Canonical absence represents zero members; an explicitly stored empty outer collection refuses.
A validated draft may clear an optional property, and application removes its key. Legacy scalar
properties become exactly one outer member only after the original bytes and hash verify.

Activation is coordinated with atomic provider blob publication. Frozen legacy verification
may be prepared while the original API remains available; new graph and seed formats must
not become the production write path until their retained payloads publish atomically with
metadata-only log events. No temporary new inline-payload event or independently committed
blob put is an acceptable activation path.

# 90. Preservation-First Store Migration

*Dated 2026-09-22. Extends §§34, 52 and 57. Migration preserves evidence; it cannot manufacture omitted history.
Implementation is unexecuted at preparation.*

Inventory and verify the source without changing it. Retain original object bytes, event records,
backend coordinates and hashes. Decode each source version with its matching original canonical
rules. Convert into a separate destination only after required input can be verified. Preserve
stable domain object identities and record old-to-new object and revision addresses.

ObjectStored/schema1 verification reads its original inline bytes under its original shape;
the new destination writes ObjectStored/schema2 metadata and the verified bytes through one
atomic blob publication. Preserve the original event's schema and body in the inventory.
ObjectRetentionRaised/schema1 keeps its original meaning on both sides. No reader guesses the
source generation from absent bytes, and an unknown or mixed shape/version refuses unchanged.

Freeze destination event occurrence identities in a migration manifest before writing, binding
each to a source event coordinate. Retry an interrupted conversion from that same manifest.
Events absent through old content deduplication cannot be reconstructed from a later state.
A hash is not its operation payload, and a retracted legacy record is not its prior acceptance.

If operation payloads, governing ontology, validation attribution, prior acceptance or retained
evidence cannot be verified, refuse with the exact missing evidence and affected record. Preserve
the source; do not reset its lineage, fill in synthetic metadata, or treat a seed snapshot as
lost history. New-format admission is not an implicit conversion path.

Publish or switch to a destination only after complete mapped-history verification and restart
equivalence through the real kernel authority. Unsupported conversion may finish with a named
refusal and unchanged source; it must not be reported as a successful migration.

# 91. Durable Kernel Authority, Retained Decisions and Document Commands

*Dated 2026-09-22. Completes §§8,19–21,34,56–57 and amendments88–90. This is the contract for
activation, not evidence that the implementation already passes it. It adopts the coordinator's
durable-record decisions and document-input decision; the earlier writer preparation is subordinate
where it differs. Original production codecs remain available during preparation. Activation of
the new graph, seed, receipt and writer contracts is one coordinated change behind atomic provider
blob publication.*

## 91.1 Registered authority and the P1 validation profile

The host supplies a complete registered authority state from trusted bootstrap configuration.
An agent id does not acquire authority because a seed document or proposal names it. Both the
bootstrap operator and validator are registered, distinct agents. The validator in the actual
BootstrapContext must equal the retained profile's validator. Names and capability strings are
retained metadata; this does not introduce P5 trust scores or a capability authorization language.

AuthorityStateV1 has exactly format "ekr.authority-state/1", agents, and validation_profile.
agents is a map keyed by AgentId, and each entry contains id, name and a set of capability strings;
the key must equal id. The profile has exactly these fields:

| field | P1 value |
|---|---|
| format | "ekr.p1-validation-profile/1" |
| ruleset | "ekr.p1-deterministic/1" |
| checks | Structural, Reference, Type, Cardinality, OntologyConstraint, Provenance, Authorization, in this order |
| validator | the actual registered validating AgentId |
| proposer_separation | "distinct-authenticated-actor/1" |
| provenance | "retained-admissible-evidence/1" |
| application | "ekr.p1-apply/1" |

An omitted, reordered or substituted check, unsupported profile, unknown agent, or contradictory
validator refuses. This fixed profile runs the seven deterministic checks and verifies the actual
retained payload bytes every canonical evidence dependency cites. Presence of an EvidenceId or
a content hash alone is insufficient. Existing graph-assertion/observation dependency integrity
and bootstrap refusals remain binding. No new confidence threshold, external-source trust policy
or unsupported constraint-expression meaning is inferred.

Authority is retained, not reconstructed from whichever startup flags are supplied later.
Reopen verifies the trusted bootstrap anchor against the complete retained state, then uses that
state and profile for history. P1 does not replace the registry or profile through a new startup
argument. A later change requires an explicit versioned mutation and policy.

## 91.2 The four roots and retained seed context

ontology_root is the value-domain hash of the complete loaded ontology: schema version id,
number, parent and created_at; every node and edge declaration, including unused declarations;
all names, parents, property definitions, recursive value types, cardinalities, required flags,
constraint strings, lifecycle declarations, operation arguments/preconditions/transitions/emits,
endpoint restrictions, inverse/symmetric/transitive flags. Declaration collections are ordered
by stable ids. There is one declaration encoding shared with canonical transaction encoding,
so a field cannot contribute to one address and vanish from the other.

agent_root is the value-domain hash of the full AuthorityStateV1, including its exact validation
profile. Encode format, agents and profile in declaration order; map/set members use canonical
ordering, while the checks vector retains its specified order. knowledge_root hashes the full
node, edge and assertion collections using the new property and assertion shapes. evidence_root
hashes the complete retained Evidence collection, with all cited payload addresses resolved and
verified before admission/replay. A defined empty collection can have a real empty root; a
populated ontology or authority registry cannot be represented by a zero placeholder.

SeedEnvelope/2 has exactly format "ekr-seed-envelope/2", input, context, authority and committed_at,
in that declaration order. input is the complete Seed/2 document defined in §89: format,
ontology, the complete GraphDocument/2 envelope, and evidence_payloads. context retains the
actual BootstrapContext {operator, validator}. authority is AuthorityStateV1 from the host, not
a field accepted from the input document. committed_at comes from trusted execution context.

Bootstrap validates the declared ontology and proposed graph under the fixed profile before
any publication. It applies the existing kernel Proposed-to-Accepted bootstrap attribution with
the actual validator. Retain the original proposed input, context and authority needed to reproduce
that transition; do not rewrite the submitted seed into a falsely caller-accepted seed.

Revision0 has revision number0 and no parent. Its transaction field addresses the retained seed
envelope, keeping the seed meaning distinct from later accepted transactions. Seed initialization
takes one complete Seeded occurrence and its SeedResultV1, envelope and required evidence payloads.
It atomically publishes all required blobs and metadata. Verify occurrence kind, seed address,
revision id, EventId, result-record address and the computed Root0 together.

For a retry, first parse and compare the complete Seed/2 input with the retained typed input,
and compare the actual BootstrapContext and trusted host AuthorityStateV1 anchor with the
retained context and authority. This comparison precedes allocation of a new occurrence or
timestamp. Matching logical input and actual trusted context return the original SeedResultV1,
including its original Root0 and committed_at, without an event or object write, even after
ordinary commits advance the head. Different input or anchor returns AlreadySeeded without a
write. YAML whitespace alone does not change the parsed seed. This seed comparison does not
relax §91.3's separate requirement to retain exact transaction-document bytes.

## 91.3 Exact transaction documents and durable proposals

Propose consumes one exact UTF-8 YAML document:

```yaml
format: ekr.transaction-document/1
transaction: # the full typed GraphTransaction<Value>
  id: <TransactionId>
  proposer: <AgentId>
  operations: <typed operation vector>
  evidence: <EvidenceId set>
```

The notation above describes typed fields; it is not an executable fixture. This is the first
transaction-document envelope, not a version2 graph document. Use the established
serde_yaml_ng0.10 parser family used by SeedDocument::from_yaml. Operations use its typed YAML
tags, such as !CreateNode; an untagged JSON enum object is not an alternative operation encoding.
JSON-compatible scalar and collection syntax is accepted only where the typed YAML grammar
admits it. Exactly one document is admitted. Unknown formats,
unknown semantic fields at any nesting, duplicate semantic/map keys, mismatched enum payloads,
invalid UTF-8 and malformed or empty-operation documents refuse Propose without a Proposed fact.
User Record keys remain user data, not schema fields.

The format selects a frozen inclusive parser profile: at most 262144 input bytes, depth32,
32768 expanded representation nodes, 4096 entries per map or sequence, 65536 decoded UTF-8 bytes
per string value, 4096 bytes per key and 1048576 total expanded string bytes. Operations contain
1 through256 elements; the evidence input contains at most1024 elements. Checked counters charge
each alias at its expanded position and content. Caller documents cannot change the profile.

Apply the raw-byte cap before parsing, copying or unbounded reading. A budgeted representation
pass checks shape and expanded allocation before strict typed decoding of the original bytes.
Do not convert through a generic Value and silently coerce string semantics. Semantic fields and
actual decoded map keys reject duplicates before decoding their repeated value. Required maps
and sequences require actual container syntax; an empty plain scalar is not an empty collection.
Unique Record keys remain data, including reserved-looking names and merge-key text; do not apply
YAML merge processing. Preserve explicit empty inner Lists and Records.

The pinned loader buffers YAML events before representation visitation. The byte cap bounds its
input; the visitor limits expanded application values. This is not an exact allocator-byte limit
or pre-loader event/scalar quota. Proposal, validation and replay all select this profile from
the document format. A stricter host upload cap affects new ingress only, never historical
validity. A changed frozen profile requires an explicit document version decision. Limits refuse
with named reasons, never truncate or coerce. All exact-boundary and over-limit cases must execute
before exposing the command; these requirements do not assert that implementation already exists.
The strict parser preserves well-formed transient values, including nonfinite Float spellings,
for the actual Type validator to refuse. Parser acceptance is not canonical eligibility.
Other schema-valid semantic violations likewise persist Proposed and reach Validate.

Retain the exact supplied document bytes, including whitespace and nonfinite-value spelling;
document_hash is ContentHash::of_bytes over those bytes. Never reserialize through JSON to
create an alleged original input. A typed convenience API may encode a documented lossless
YAML document and submit those actual generated bytes through the same parser/path, but cannot
claim they were the caller's original document.

Trusted execution context supplies submitter and submitted_at once. transaction.proposer and
every AddAssertion.proposed_by must match that authenticated submitter; caller input cannot
forge the actor. Derived transaction id, operation count and evidence-set hash must agree with
the reparsed document. Canonical transaction/operation hashes are present only when derivable
without altering the proposal; no fabricated hash stands in for a refused Float.

Every receipt and record below is a strict versioned retained payload with no unknown fields,
no duplicate keys, no mixed versions and no self-authorizing deserialization. Their payload
addresses use ContentHash::of_bytes over the exact retained record bytes. These addresses are
distinct from value-domain addresses of canonical transaction or validation material.

## 91.4 Complete records for all six occurrence kinds

The following tables give fields in declaration order. An Optional value is explicitly optional;
all other fields are required. Lists whose type is a set retain canonical set order; ordered
issue and operation vectors retain their actual order. Integer counts must fit their Rust
unsigned domain and agree with the values they count.

| record | complete fields |
|---|---|
| ProposalRecordV1 | format="ekr.proposal-record/1", event_id:EventId, submitted_at:Timestamp, submitter:AgentId, document_hash:ContentHash, document_bytes:Bytes, transaction_id:TransactionId, operation_count:u64, evidence_hash:ContentHash, canonical_transaction_hash:Optional<ContentHash>, canonical_operations_hash:Optional<ContentHash> |
| ValidationBasisV1 | format="ekr.validation-basis/1", graph_root_id:GraphRootId, previous_revision_id:RevisionId, previous_event_id:EventId, previous_record_hash:ContentHash, previous_root:Root, previous_root_hash:ContentHash, seed_hash:ContentHash, ontology_root:ContentHash, authority_root:ContentHash, validation_profile_hash:ContentHash |
| ValidationReceiptV1 | format="ekr.validation-receipt/1", event_id:EventId, proposed_event_id:EventId, proposal_record_hash:ContentHash, transaction_hash:ContentHash, operations_hash:ContentHash, evidence_hash:ContentHash, operation_count:u64, basis:ValidationBasisV1, validators:Set<AgentId>, validated_at:Timestamp, validation_hash:ContentHash |
| CommitReceiptV1 | format="ekr.commit-receipt/1", event_id:EventId, revision_id:RevisionId, proposal:ProposalRecordV1, validation:ValidationReceiptV1, validation_record_hash:ContentHash, committer:AgentId, committed_at:Timestamp, result:Root, result_hash:ContentHash |
| SeedResultV1 | format="ekr.seed-result/1", event_id:EventId, revision_id:RevisionId, seed_hash:ContentHash, authority_root:ContentHash, committed_at:Timestamp, result:Root, result_hash:ContentHash |
| RejectionRecordV1 | format="ekr.rejection-record/1", event_id:EventId, proposed_event_id:EventId, proposal_record_hash:ContentHash, requested_basis:ValidationBasisV1, validator:AgentId, rejected_at:Timestamp, issues:Vec<ValidationIssue> |
| StaleRecordV1 | format="ekr.stale-record/1", event_id:EventId, validation_record_hash:ContentHash, expected_basis:ValidationBasisV1, observed_revision_id:RevisionId, observed_event_id:EventId, observed_record_hash:ContentHash, observed_root:Root, observed_root_hash:ContentHash, stale_at:Timestamp |

Root retains all seven fields: revision, parent, ontology_root, knowledge_root, evidence_root,
agent_root, transaction. parent is the prior complete Root hash, not merely its knowledge root.
ValidationIssue retains id, transaction_id, validator check name, code and message; assign
IssueIds once when recording the result, outside deterministic pure checks. An assertion's
Rejected assessment retains IssueIds rather than kernel ValidationIssue objects, preserving
the graph-to-kernel dependency direction.

The event/2 envelope's record_hash selects exactly this record for its existing payload kind:

| kind/index | retained record | metadata payload |
|---|---|---|
| Seeded/0 | SeedResultV1 | revision_id, seed_hash |
| TransactionProposed/1 | ProposalRecordV1 | transaction_id, proposer, operations_hash:Optional<ContentHash> |
| TransactionValidated/2 | ValidationReceiptV1 | transaction_id, against, validation_hash |
| TransactionRejected/3 | RejectionRecordV1 | transaction_id, issues count |
| TransactionStale/4 | StaleRecordV1 | transaction_id, validated_against, current |
| RevisionCommitted/5 | CommitReceiptV1 | transaction_id, revision_id, number, knowledge_root |

Event names and canonical variant indices remain the existing six; event/2 is still unpublished
and is the only new event version in this change. Domain EventId, provider event id and atomic
group attempt id are separate identities. All duplicated identities, counts, roots and addresses
must agree. For Proposed, proposer is the trusted submitter and operations_hash is the optional
canonical_operations_hash; absence does not prevent recording a well-formed invalid proposal.

Proposed and Validated decisions survive restart independently. Commit embeds their full retained
records and verifies their addresses against the separately retained originals. This bounded
duplication allows pure replay authority without nested reads through a provider transaction.
It does not authorize either record merely because its self-hash verifies. Rejected and Stale
retain the complete terminal decisions but create no canonical revision.

## 91.5 Validation binding, application and replay authority

ValidationMaterialV1 is exactly format="ekr.validation-material/1", the full accepted canonical
transaction, ValidationBasisV1 and the exact actual validator set. Its address is
ContentHash::of(material), in the value domain. The fixed P1 profile permits its actual registered
validator, not a caller-supplied subset or an in-memory receipt allowlist. ValidationReceiptV1's
separate payload address binds validated_at and occurrence identities.

The basis binds the complete prior root and its hash, graph identity, seed envelope address,
ontology and authority roots, exact profile hash, prior revision/event identities and prior
SeedResult/CommitReceipt address. The previous record address matters because Root itself omits
time and bootstrap context. Equal revision numbers or equal knowledge roots in different lineages
cannot authorize interchange. Root.transaction on a later revision is the full accepted
canonical transaction hash, not the operation-vector hash.

Validate first requires the named revision to exist for a Proposed transaction. An absent
revision returns RevisionNotFound, leaves the transaction Proposed and writes no Validated or
Rejected occurrence, receipt, object or issue. There is no complete requested_basis to retain
for an absent revision; never manufacture one. An older existing revision remains a valid
validation basis. If the canonical head has moved from that complete basis, Commit records
Stale according to its normal canonical-head check rather than refusing historical validation.

Every admitted operation has one deterministic application, or refuses with a named reason before
sealing. CreateNode, UpdateProperty, CreateEdge, DeleteEdge, AddAssertion and supported Invoke
operate on the complete post-state; property values preserve §89's outer multiplicity.
Retraction names the assertion and reason. Supersession names the replaced assertion, replacing
assertion and effective valid-time boundary. Retraction retains its existing operation index5
with the new payload; SupersedeAssertion is appended after Invoke at index11.
Schema-changing operations and MergeEntity remain explicitly refused in P1.
Unsupported preconditions/effects refuse without disabling a valid declared lifecycle transition.

Application treats the admitted transaction as an unordered atomic set, including candidate
assertion acceptance and supersession eligibility in §88. The submitted operation-vector order
remains part of its canonical bytes; unordered application does not silently repair historical
or current hash encodings. Conflict detection and validation must yield the same admission and
resulting state under operation permutation, while the transaction address can differ.

Replace the boolean replay attestation with a fallible kernel-owned admission/apply authority.
Only the kernel validates and constructs admitted canonical state and roots. Given the prior
admitted state and verified retained inputs, replay checks exact versions and addresses, document
parser results, all receipt/basis/occurrence linkages, registered actors and exact profile, actual
evidence bytes and timestamps. It reruns the same pure validation and application and recomputes
all four subroots, full root and result address. It does not substitute today's ontology/profile
for a retained historical one. Unsupported historical rulesets or missing inputs produce a named
refusal, not a best-effort fold.

The storage layer preserves generic bytes and coordinates without learning kernel receipt
semantics or constructing authority capabilities. Do not implement nested authority-to-store
reads inside a provider transaction. Corrupt commits refuse reopen; they do not silently disappear
from the fold or return the seed as if it were the latest revision. Historical reconstruction
stops at the selected committed revision and preserves its own schema, authority and graph.

## 91.6 Trusted time, occurrence idempotency and atomic publication

Resolve each trusted actor, domain occurrence id and timestamp once per logical decision.
For a committed transaction submitted_at <= validated_at <= committed_at. Committed timestamps
are nondecreasing along canonical lineage; equal times are legal and revision numbers order ties.
The affected assertion recorded_from bounds in §88 also hold. Retry and replay reuse retained
timestamps, never a newly sampled clock. Terminal refusal times likewise come from trusted context.

Each successful seed retry returns that SeedResultV1's recorded Root0, even if another transaction
has since advanced the head. Each successful commit retry first resolves its exact occurrence
and returns its own retained CommitReceiptV1 and result without another event or state change.
The public Commit input is the same transaction_id on retry. A retained Committed transaction
therefore succeeds silently, even after head advancement; Proposed, Rejected and Stale
transactions still return TransactionStateConflict. Retained input/context and success are
resolved before allocating an occurrence or sampling time. Seed uses the parsed-input and
trusted-anchor comparison in §91.2, not exact YAML byte equality or a newly timestamped envelope.
Reuse of an EventId with different bytes refuses. Provider
contention does not allocate another domain occurrence or alter timestamps/receipts. Provider
group attempts obey the provider's exact fingerprint/idempotency contract.

Canonical head and provider stream position are different facts. On conflict, first resolve
whether the exact immutable occurrence already committed, then compare the complete validation
basis with the actual canonical head. Unrelated object/proposal/validation records can permit a
bounded retry of the same immutable occurrence. A competing canonical commit makes the accepted
transaction Stale and records the full stale decision. An unknown publication outcome triggers
exact occurrence reconciliation; never delete source data or blindly republish under a new id.

All seed, proposal, validation, commit, rejection and stale record bytes, evidence payloads and
required content objects publish through the provider's atomic blob-and-metadata path. Revision
and object log events carry only permitted ids, hashes, counts, times and status metadata, with
payload bytes behind verified blob bindings. Neither inline numeric byte arrays nor independent
blob-put followed by event append satisfies this contract. A metadata-only ObjectStored retains
content_hash, storage_class, byte_len and stored_at under backend schema2; the atomic provider
binding resolves the bytes. ObjectStored/schema1 remains the frozen inline verification/migration
format; ObjectRetentionRaised remains schema1. Enforce §89's strict per-event dispatch.
The EKR object identity/hash scheme and the provider's blob address are checked explicitly and
must not be treated as interchangeable by assumption.

## 91.7 Command inputs, projections and acceptance

The P1 CLI remains the six verbs Seed, Propose, Validate, Commit, Snapshot and Explain.
Seed takes seed_document:SeedDocumentPath; Propose takes
transaction_document:TransactionDocumentPath. The shared command parser loads the exact document,
derives hashes/counts and binds execution identity; callers do not supply authoritative hash strings.
The ESS Transactions view reads every retained transaction state, including terminal states,
from real durable records. It is not a seventh CLI verb and not an adapter-local map.

ESS projects strict tagged Rust variants through a named kind plus typed optional payloads where
a no-payload variant cannot be expressed by an ESS empty struct. Exact payload/kind coherence
remains a runtime projection obligation. Canonical graph values project their exact canonical
binary bytes and canonical kind; transient transaction values remain the full typed YAML document.
This neither converts Float to Decimal nor changes the strict source wire representation.
The complete admitted seed and ontology projections retain all semantically relevant fields.
Compiler acceptance of these declarations does not establish runtime agreement.

The ESS declaration uses ess/7 for command-local retained-result replay and effect-free default
state refusals. Seed's retained-seed outcome replays seeded using the original emitted revision
identity; Commit's retained-commit outcome replays committed using its original transaction input.
Validated selects the committing transition, Committed selects silent replay, and the default
named refusal covers Proposed, Rejected and Stale. External stale publication remains distinct.
Every emitted field has a declared input/response source or explicit generated ownership.
Generated means the real kernel supplies the derived value from retained inputs and trusted
execution context; a target cannot fill it from an expected conformance result. Seed and Commit
return their actual retained SeedResultV1 and CommitReceiptV1, including on silent retries.
The unfiltered Revisions view observes every retained Revision field and state, including the
original seed after a later head. Transactions observes every retained transaction state. These
are real kernel query surfaces, never expected-response maps or additional CLI verbs. The actual
conformance adapter preserves typed Integer values losslessly from handler results before
comparison; any JSON bridge must exact-decode them or report Unsupported. Native typed parity
does not certify arbitrary raw numeric spellings through a floating-point JSON conversion.
Adoption requires a verified released compiler supporting this declaration and measured synthesis;
until then this remains an unactivated draft. Generated immediate retries do not establish
later-head, restart, semantic seed comparison or no-physical-write behavior; authored executions
must supply that evidence through both real providers.

For conformance fixture setup, a target may stage known synthetic documents at the exact
synthesized relative paths, within an isolated allowed fixture root, refusing absolute paths,
parent traversal, symlink escapes and arbitrary writes. It must call the real shared parser and
kernel with authenticated role execution identity, preserve exact scenario inputs, and observe
the real returned result. It cannot rewrite a synthesized hash/path/actor/revision and echo the
original, forge ids to satisfy role linkage, or fabricate retained transaction state.
Authored scenarios supplement rather than replace generated obligations.

Before claiming activation, execute the immutable generated and authored suites against the real
target with no unaccounted skips or unsupported/error scenarios. The first durable acceptance is
seed -> propose a new node and evidence-backed assertion -> validate -> apply -> atomically persist
-> restart in a fresh process -> query changed knowledge, with both real providers and real kernel
authority. It must exercise a Many property, a single List-valued property and a changed knowledge
hash. Then cover stale/concurrent/interrupted publication, own-result retry after head advancement,
retained Rejected/Stale state, retraction/supersession, §65's two valid-time reads and earlier-revision
reconstruction, exact source-byte retention and corruption/missing-input refusals.

Retry acceptance must use the actual public shared handlers, repeat after restart and an
unrelated canonical head advance, and compare the original receipt/result, timestamp, occurrence,
object/event counts and preserved transaction/seed data. Seed controls include semantically equal
YAML with different whitespace, a different parsed seed and changed actual bootstrap context or
host authority anchor. Commit controls include Proposed, Rejected and Stale named refusals.
Validate controls include absent-revision no-write/remaining-Proposed and older-existing-basis
success followed by Commit Stale. Provider readback must distinguish ObjectStored/schema2 from
legacy inline schema1, keep ObjectRetentionRaised/schema1, and refuse unknown or mixed versions,
missing bytes, wrong binding/address/length and unknown semantic fields without source mutation.

Required new controls include a well-formed NaN Float transaction retaining byte-identical
ProposalRecord input across restart and producing the actual named Type refusal without canonical
hashes; changing any prior receipt/profile/ontology semantic field must invalidate acceptance or
change its address as applicable. Legacy original-format immutable vectors remain unchanged.
Migration is still downstream of this real durable path and the preservation/refusal rules of §90.

---

# 92. Absent Transaction Command Targets

*Added 2026-09-22 during implementation of §91. This closes an omitted refusal;
it does not change any retained record format or transaction lifecycle.*

Validate and Commit first resolve the supplied transaction identity from verified
retained records. A well-formed identity with no retained transaction returns
`TransactionNotFound { transaction_id }`. It cannot return a state conflict with
an invented Proposed, Rejected or other state. This lookup precedes Validate's
revision-basis check and Commit's staleness check. Malformed identifiers remain
input failures; corrupt required history remains a verification failure and is
never disguised as an absent transaction.

The refusal writes no object, validation issue, receipt or event and changes no
canonical revision. Both public handlers must execute this control after a fresh
process reopen on both providers, comparing object/event counts and the complete
retained state before and after. These controls are unexecuted at declaration.
The corresponding ESS outcomes are `transaction-not-found`; generated compiler
obligations alone do not establish runtime behavior.

---

# 93. Durable Command Response Values

*Added 2026-09-22 before ordinary durable handlers are implemented. These are
command response values over the existing §91 records, not new persisted formats.*

Propose returns its actual retained ProposalRecordV1. Validate returns
ValidationCommandResult, exactly Validated(ValidationReceiptV1) or
Rejected(RejectionRecordV1). Commit returns CommitCommandResult, exactly
Committed(CommitReceiptV1) or Stale(StaleRecordV1). The corresponding ESS unions
use a kind discriminator. A retained successful Commit returns Committed with
its original receipt, without another occurrence or clock sample.

A recorded rejection and a recorded stale decision are declared command outcomes.
They are distinct from a named refusal that records nothing and from an
operational or retained-history verification failure. Neither the kernel host
nor a presentation adapter may manufacture a success receipt for another branch,
replace actual rejection issues with a count, or reconstruct an alleged original
proposal through a JSON round trip.

The shared-handler and later CLI/conformance acceptance must bind each branch
to its actual retained record after restart. A well-formed noncanonical Float
proposal must return its exact document bytes and absent canonical hashes, then
its real recorded Type issues. A stale Commit must return its actual stale basis
and observed head. These response controls are unexecuted at declaration.

The trusted provider opener also preserves the typed distinction between a
valid but different host anchor and corrupt retained state. Seed maps the former
to AlreadySeeded as §91.2 requires, without admitting or exposing state under the
changed anchor; ordinary commands and reads refuse it. Recovering the ontology
from the retained seed does not authorize replacement host configuration.

---

# 94. Durable Publication Preparation and Recovery

*Added 2026-09-22 after the writer's executable unknown-outcome probe. This
implements the cross-call and restart obligation in §91.6; ordinary revision,
receipt, graph and seed formats retain their versions.*

`unresolved_publication_cannot_be_replaced_by_a_new_occurrence` reproduced the
loss of a prepared occurrence after UnknownCommit: a later Commit sampled a new
time and minted new occurrence/revision identities. The probe wraps a real SQLite
store at the kernel port; it is not native interruption evidence. The independent
publication recovery review also found that retaining Publication alone loses
the provider request's exact object appends, expectations and attempt identity.
The current Eventlog atomic-group port resolves an uncertain result only by
retrying that original request. A successful empty history read is no absence
fence for an in-flight request.

## 94.1 Selection before publication

Use a private Eventlog-backed preparation journal, with no filesystem sidecar or
second persistence engine. A command first looks up its preparation slot, before
sampling a clock. The slot key is Bootstrap for the entire tenant lineage;
Propose plus transaction identity; Validate plus transaction identity and the
retained proposal occurrence/address; or Commit plus transaction identity and
the retained validation occurrence/address. Input, actor, basis and seed hashes
do not create additional slots for competing inputs to the same logical command.

The selected record separately binds a value-domain logical input hash. Seed
binds its parsed complete input, context and actual authority anchor; Propose
binds exact document bytes and trusted submitter; Validate binds the transaction,
retained proposal, requested basis and actual validator; Commit binds the
transaction, retained validation and actual committer. Every command also binds
the actual host context/authority. The input hash excludes freshly sampled time
and identities. Different input cannot resume an elected decision.

Elect the initial immutable preparation using Expected::NoStream on that slot.
Retain its complete decision and initial native request in one atomic private
blob-and-selection-metadata group. Publish the domain occurrence only after
acknowledging that selection or reading and verifying its actual winner. A
preparation with UnknownCommit grants no publication authority. A retry may
compete only for the same conditional slot and must adopt its matching winner;
it cannot publish a local candidate because a read returned empty. Losing
preparations commit neither their metadata nor private blob binding.

Unelected candidates may allocate IDs/time that are discarded. Once elected,
the actual actor, timestamp, occurrence/revision identities, record bytes and
result are immutable. This qualifies §91.6's once-per-decision rule: it refers
to the elected decision, not every unsuccessful conditional candidate.

## 94.2 Exact native attempts

The private format is `ekr.publication-preparation/1`. It carries command_key,
input_hash, decision, attempt_number, previous_attempt_hash, native_request and
native_fingerprint. Its strict typed carriers are declared in the store ESS.
The decision contains the complete RevisionEvent/2, staged object bytes with
their retention/time and expected revision-stream version. The native request
contains tenant, ordered stream appends, exact expectations, complete NewEvents,
every CommandMeta field and ordered blob digest/byte bindings. Preserve provider
time's nanoseconds and offset losslessly. Atomically appended groups require
claim=None; a present claim is refused. Event data is retained as exact JSON
object bytes, decoded without integer rounding or duplicate-key loss. Validate
the reconstructed request with the provider's actual fingerprint algorithm and
compare the retained fingerprint before use. No fields are regenerated.
The native request must also correspond to the elected decision: the actual
tenant, authorized EKR streams, domain event, object metadata, retention changes,
blob set and writer metadata must agree. A recomputed fingerprint authenticates
no caller by itself. Reject extra appends, foreign streams, unrelated blobs and
changed metadata even if a forged record supplies a matching new fingerprint.

The first attempt has no predecessor. Every successor is a conditional append
at the preceding slot version, binds its predecessor address and increases the
attempt number. Missing, malformed, mismatched or unsupported journal records
refuse recovery. The private selection metadata uses
`ekr.store.PublicationPrepared` with backend schema1 and carries only the
preparation address and attempt-chain coordinates. Its blob key is in a
distinct private namespace; it creates no public ObjectStored record or
canonical object lookup binding. Journal records never replace kernel admission.
Every elected decision is still checked by the real kernel authority before
new publication; a forged preparation is not a ValidatedTransaction.

Resume the exact unresolved native attempt before reconsidering canonical
staleness or rebuilding object appends. UnknownCommit retains the same attempt.
Only retrying that exact request and receiving a definitive conflict authorizes
a conditional successor; a changed history length alone does not. With an
unchanged canonical basis, retain the domain occurrence/time and prepare a new
native attempt. With a competing canonical winner, a distinct Stale decision
may follow the resolved unsuccessful Commit and retains its original sampled
time. An unresolved Commit can never be converted into Stale.

## 94.3 Outcomes, visibility and acceptance

Confirmed publication of the corresponding decision resolves recovery, including
Proposed, Validated, Rejected and Stale decisions which create no canonical
revision. Preserve existing public retry behavior: Seed and Committed return
their original retained success; other existing terminal/state refusals stay
as declared. A pending slot with different input returns typed operational
publication conflict/uncertainty. It is not an invented transaction state or
AlreadySeeded before a seed is published. Named no-write input/state refusals
are decided before creating a preparation. Read commands never recover by
writing. Retained success is resolved before clock sampling or journal writes.

Private preparation is retained recovery information, not canonical knowledge
or a public object; canonical record/object publication remains atomic. The
maintenance path must not erase unresolved preparation or payload needed for
exact reconciliation. An exact successful retry must not restore erased public
bindings. Do not delete data to clear an uncertain result.

Required controls, unexecuted at declaration except the original red probe:
unknown/crash before and after preparation acknowledgement, publication and
resolution; concurrent same-slot preparers; different bootstrap inputs sharing
one slot; restart with identical native fingerprint, actor, IDs, time and receipt;
unrelated object/proposal movement; canonical head advancement during uncertainty;
missing/corrupt private bytes; input mismatch; exact retry following erasure;
and zero committed bindings from losing preparations. Exercise both native
providers, all decision kinds and real kernel authority. Fault wrappers must be
labelled separately from actual provider interruption evidence. Preserve the
original red and require its unfiltered passing result before writer closure.

---

# 95. Schema Evolution Transactions

*Added 2026-09-26 by wave p5-01 (`story:schema-evolution-transactions`, parts B and C). Extends §§19,
20, 26 and 91.5. It adds validation profile v2 beside the P1 profile; it changes no retained v1
record and moves no retained transaction encoding: `ModifyProperty` gains an owner, and its P1
shape still reads and encodes as it did.*

A committed transaction can add a node type (`DefineNodeType`), add an edge type
(`DefineEdgeType`), and add or redeclare a property on a type the ontology declares
(`ModifyProperty`). The commit produces the next schema version. Case names below are in
`crates/ekr-kernel/tests/schema_evolution.rs` unless marked `replay:` (in
`schema_evolution_replay.rs`, which runs both providers).

1. **Schema-only transactions.** A transaction that mixes a schema operation with an operation of
   any other kind is refused by the structural validator with `mixed-schema-transaction`. Mixed
   transactions are a later milestone. Executed by
   `a_schema_change_mixed_with_another_kind_is_refused`.
2. **Two profiles, fixed at seed.** Validation profile v2 is ruleset `ekr.p2-deterministic/1` with
   application `ekr.p2-apply/1`, beside v1's `ekr.p1-deterministic/1` with `ekr.p1-apply/1`. Each
   is one exact value; a pair mixing the two is refused (`seed-authority-profile`). A store keeps
   the profile its authority anchor names at seed, because the anchor is part of the seed envelope
   and is compared again on every reopening. No mechanism moves a store from v1 to v2; that is a
   later milestone. Executed by replay: `a_profile_mixing_the_two_rulesets_is_refused` and
   `a_store_keeps_the_profile_it_was_seeded_under_on_both_providers`.
3. **The version field.** `GraphTransaction` gains one optional field, `schema_version`, carrying
   the new `SchemaVersionId` (from `ekr mint schema-version`). It is required exactly when a schema
   operation is present: its absence is refused with `schema-version-missing`, and its presence
   without one with `schema-version-without-schema-change` (under either profile). When absent it
   is absent from the canonical encoding, so every transaction without it encodes byte-identically
   to before; when present it is written last, as a tagged `Some`. In `ekr.transaction-document/1`
   it is one optional key, and a document without it reads as before. Executed by
   `the_version_id_is_required_exactly_when_a_schema_change_is_present`,
   `a_transaction_without_a_version_id_encodes_byte_identically_to_before` and
   `the_transaction_document_takes_one_optional_version_key`; that no retained vector moves is held
   by the existing `current_vectors`, `current_validation_hash` and `current_root_sensitivity`
   cases.
4. **Lineage.** `GraphRoot.schema_version_id` stays the seed's: it is the root's identity. The
   ontology root carries each new version, with `number` one more than the prior version's,
   `parent` the prior version, and the commit time as `created_at`. A new version id must appear
   nowhere on the lineage: `Ontology::evolve` sees the prior version and its parent, and the kernel
   holds it against every version its retained revisions name, refusing with
   `schema-version-reused`. Executed by `a_version_id_already_on_the_lineage_is_refused` and replay:
   `replay_reproduces_every_root_across_schema_versions_on_both_providers`, which reuses the seed's
   id two versions later.
5. **`ModifyProperty` carries its owner.** Its payload is `{ owner, property: PropertyDefinition }`
   (`ekr.kernel.PropertyModificationProjection`), in the payload, the canonical encoding (a tagged
   `Some(owner)`, then the declaration), the transaction document and `ekr.kernel`'s operation
   projection. The P1 shape — the bare declaration, which P1 refused and v1 stores retain — is
   frozen in the same variant as `owner: None`, the way `crate::legacy` freezes the original
   format: it reads from and writes to the bare declaration and encodes as the declaration alone,
   exactly as at the wave's base. A mapping mixing the two shapes is refused. Profile v2 refuses
   the P1 shape with `modify-property-without-owner`. The encodings of the other eleven kinds do
   not move. Executed by `the_modify_property_owner_reaches_the_encoding`,
   `an_ownerless_modify_property_is_refused_under_profile_two`,
   `crates/ekr-kernel/tests/encoding_field_order.rs`, `crates/ekr-kernel/tests/validation.rs`'s
   `the_twelve_current_operation_numbers_are_the_domains_and_the_declarations`, and — against
   bytes the base kernel wrote — `crates/ekr-kernel/tests/base_era_v1_replay.rs`'s
   `the_p1_shape_encodes_exactly_as_the_base_kernel_encoded_it`,
   `every_operation_kind_in_its_base_era_shape_re_derives_its_retained_hashes` and
   `a_modify_property_mixing_the_two_shapes_is_refused`.
6. **No removal.** No operation removes a type or a property.

**Admission under v2.** The structural validator admits the three kinds as above; entity merge
stays `unsupported-operation`. The ontology-constraint validator builds the candidate with
`Ontology::evolve`, reporting each `EvolveError` under its own code (`unknown-property-owner`,
`incoherent-schema`, `schema-change-without-effect`, `schema-version-reused`), then asks
`ekr_ontology::incompatibilities` whether the candidate can replace the prior version over the
canonical graph, reporting each `Incompatibility` under its own code. It reads canonical state per
`InstanceState`: an instance of an owner is a node or edge whose own type it is; `min_values` and
`max_values` count held values only; `value_kinds` also includes the objects of active property
assertions. It stays silent where the structural validator has spoken (a type id declared twice,
or over one the ontology holds). Executed by `each_schema_change_kind_validates_under_profile_two`,
`evolve_refusals_arrive_as_ontology_issues_with_the_ontologys_codes`,
`a_change_canonical_state_would_violate_is_refused_with_a_named_issue`,
`a_value_type_change_is_refused_when_an_active_assertion_object_would_break` and
`requiring_a_property_held_only_through_assertions_is_refused`.

**Refusal under v1.** A v1 store refuses the three kinds with the issue P1 retained,
`unsupported-operation` with the message `<Kind> is not supported in P1`, unchanged byte for byte.
This is the named issue saying the store's profile does not admit schema changes. It is not
reworded, because replay compares every retained rejection's code and message with what the
ruleset says now: a new message would make every v1 store that retained such a rejection fail
to reopen. Executed by `profile_one_still_refuses_the_three_schema_kinds` and replay:
`a_v1_store_with_a_retained_schema_rejection_still_replays_on_both_providers`, which build the
rejection with this wave's kernel. That a store written *before* this wave keeps replaying is held on
SQLite, against a store the base kernel (`cee0cae`) wrote under v1 retaining a P1-shape
`ModifyProperty` proposal and its `unsupported-operation` rejection, by `base_era_v1_replay.rs`'s
`a_base_era_v1_sqlite_store_holding_an_ownerless_modify_property_rejection_still_reopens`; that
store also retains one proposal and rejection carrying all twelve operation kinds in their base-era
shapes. On the file provider the claim is unexecuted in the tree: adversary pass 1 wrote a case
over a base-era file store and it passed at `468162c`, but it is kept out of the tree because the
secret scan refuses its embedded fixture (the store's idempotency keys read as generic API keys).

In one transaction under v2, two `ModifyProperty` of one property of one owner are refused as
`conflicting-write`, whether or not the declarations differ, as two writes of one field are; two
`DefineNodeType` or `DefineEdgeType` of one type id were already refused as `duplicate-identity`.
Executed by `adversary2_p5_01_kernel.rs`'s
`a_property_redeclared_twice_in_one_transaction_is_refused_as_a_conflicting_write` and
`schema_evolution.rs`'s `a_type_defined_twice_in_one_transaction_is_refused_under_profile_two`.

**Application and replay.** Under `ekr.p2-apply/1` a committed schema change replaces the graph's
ontology with the evolved version; nodes, edges and assertions are unchanged. Replay revalidates
each retained decision under the store's profile, against the ontology as of the revision it was
validated against, and with the lineage of the revisions up to that one. Seed, schema change, data
using the new types, a refused incompatible change, a second schema change and a refused lineage
reuse reopen with identical roots, graphs and records on the file and SQLite providers. Executed by
replay: `replay_reproduces_every_root_across_schema_versions_on_both_providers`.

---

# 96. Store Performance: Compact Records, One Replay, Replay Checkpoints

*Added 2026-09-26. Extends §§ 34, 91.3–91.6 and 94. It adds three retained format versions beside
the originals and one private cache format; it changes no canonical root, no validation rule and
no `ekr.transaction-document/1` limit. Every `/1` record is still read and re-encodes to its original
bytes, so every retained address holds.*

**What was measured.** A file store of 65 revisions, 2,257 nodes, 5,727 edges and 7,190 assertions
(397 log frames, 426 blobs, 361 MB). `ekr head` took 5.1 s and `ekr snapshot` 7.7 s; one propose,
validate and commit of a 250-operation document took 25, 28 and 25 s. Each open replayed the whole
lineage — every proposal re-parsed three times, every transaction validated twice, every revision's
graph re-applied and its knowledge root recomputed — and one command did so eight times: once to
read its state and twice in each of three preparation authorizations (§ 94.2). Of the 361 MB,
309 MB were publication preparations: each byte string was a JSON array of decimal numbers, and a
preparation held each staged object twice, in `decision.objects` and in `native_request.blobs`.
The File provider hashes every retained blob when it opens (eventlog-file `fe8a0a7`), so the bytes
alone cost 1.3 s per verb.

## 96.1 Compact records

`ekr.proposal-record/2` holds the fields of `/1`; `document_bytes` is one standard padded base64
string (RFC 4648 § 4) where `/1` writes a number array. `ekr.commit-receipt/2` holds the fields of
`/1` and embeds the transaction's retained proposal in whichever of the two formats it was retained
in; a `/1` receipt embeds only a `/1` proposal. `ekr.publication-preparation/2` writes every byte
string as base64 and omits `native_request.blobs`: § 94.2 already required that list to be exactly
the decision's objects in address order, so a `/2` reader rebuilds it, and the native fingerprint
is still taken over the complete request. New records and attempts are `/2`. Each format admits only
its own spelling of each byte string, and base64 only in its one canonical spelling
(`ekr_core::bytes`), so a record has one byte form. An extra native blob binding, refused by name
as `preparation-blob-set` in `/1`, is unrepresentable in `/2` and refused there as
`preparation-fingerprint`. Executed by `crates/ekr-kernel/tests/current_records.rs`'s
`each_proposal_format_admits_only_its_own_document_spelling`, the `proposal/2` and `commit/2` pins of
`current_vectors.rs` beside the unchanged `/1` pins, `crates/ekr-store/tests/current_vectors.rs`
(the `/1` layout of an elected attempt is byte for byte the previous release's record) and
`crates/ekr-kernel/tests/replay_checkpoint.rs`'s
`retained_records_and_preparations_hold_each_payload_once_as_base64`.

The seed envelope (`ekr-seed-envelope/2`) is unchanged: its evidence payloads are still number
arrays, because the envelope embeds the `ekr-seed/2` input as given.

## 96.2 One replay per command

A replay state is identified by the prefix it covers: a chain digest over each occurrence's stream
position and complete domain event, which carries the payload address of every record replay reads
(`ekr.replay-prefix/1`). Replay is deterministic in exactly those inputs under one host context and
anchor, and retained objects are content-addressed and checked on load, so two histories with one
digest reach one state. The kernel authority keeps the few newest states it reached and continues a
replay from the longest cached prefix; a candidate replayed before publication and the same
occurrence read back after it share a digest, because the provider's event identity is not an input
of replay. Within a replay each retained document is parsed once, and a commit whose basis is the
revision its validation was computed against reuses that validation's sealed result: validation is
a pure function of the document, that revision and the lineage before it. The store handle keeps
the objects it verified and rereads one only after writing it. Nothing is reused across processes
by this cache.

## 96.3 Replay checkpoints

`ekr.replay-checkpoint/1` is private cache data, never a canonical object and never an authority: the
state one complete kernel replay reached, reduced to what the retained records do not already say —
the head revision's graph (as an `ekr.graph-document/2`), each schema version from the revision it
came into force at, the seed's evidence payload addresses, the covered prefix's digest and
occurrence count, and the host context and anchor it was reached under. Everything else is decoded
again from the verified records. It lives in the private stream `ekr.checkpoint`/`canonical` as the
event `ekr.store.CheckpointWritten { checkpoint_hash, covered, binding }` and the private blob
`ekr.private.checkpoint.<checkpoint_hash>`, published in one atomic group. A kernel writes one after
it publishes a seed or a commit, and afterwards deletes the blob of the one it replaces, so one is
retained. After a proposal, a validation or a stale decision it appends only a pointer that names
the retained blob again with the new coverage. `binding` is the kernel's own value-domain address
of (host context and anchor, covered, prefix digest). Writing is best effort: a checkpoint that is
not written costs the next open time, never correctness.

**Admission.** On its first head read, a store handle offers the newest checkpoint to the kernel,
which admits it only for the exact prefix it names, under the same host, with its head graph
reproducing the knowledge and evidence roots of the head's retained receipt and each schema version
reproducing the ontology root of every revision it is in force at; the admitted state holds the
head graph and no earlier one. Replay continues from it over any later occurrences. A validation
against an earlier revision, whose graph the state does not hold, replays the whole history instead.
`ekr head` answers from the pointer alone when it names every occurrence the stream holds and its
binding is this host's for exactly that prefix: the head is then the root the head revision's
retained record carries. A checkpoint or pointer that fails any check is ignored and the history is
replayed in full, as if there were none.

**What a checkpoint takes on trust.** That the prefix it covers was replayed, and every retained
decision in it re-derived, when it was written: it records that verification, it does not repeat it.
A writer able to append a self-consistent history and a matching checkpoint to the store could
therefore have an unvalidated transaction read as canonical until a full replay. `--full-replay`
(`Runtime::set_full_replay`, `EventlogStore::set_full_replay`) ignores checkpoints and replays from the
seed, re-deriving every decision, as every open did before this section. Executed by
`crates/ekr-kernel/src/checkpoint.rs`'s
`a_fresh_open_restores_the_head_and_replays_nothing_the_checkpoint_covers` and
`crates/ekr-kernel/tests/replay_checkpoint.rs`'s
`a_fresh_open_continues_from_the_checkpoint_with_the_answers_of_a_full_replay`,
`a_checkpoint_that_does_not_verify_is_ignored_and_the_history_replays_in_full` and
`validating_against_a_revision_the_checkpoint_holds_no_graph_of_replays_in_full`, on both providers.

## 96.4 Result, and what is not changed

On a store rebuilt from the same 66 transactions by this kernel (70 MB): `ekr head` 0.34 s, `ekr
snapshot` 1.0 s, one propose, validate and commit of 250 operations 0.97 + 1.00 + 1.20 s. What
remains per verb is mostly the File provider's open, which hashes every retained blob (0.31 s at
70 MB, linear in the store's size), and the first history load, which reads and hashes every
retained record. A store written before this section keeps its `/1` bytes: it opens, replays in
full until its next commit writes a checkpoint, and does not shrink.

The `ekr.transaction-document/1` profile of § 91.3 — 262144 bytes, 256 operations — is unchanged. A
build with the operation cap at 10,000 and the byte cap at 8 MiB measured 3.2 s for one propose,
validate and commit of 500 operations, 3.7 s for 2,000 and 6.2 s for 10,000 on the rebuilt store,
against about 80 s per 256 operations before this section. Raising the caps is a new document format
version with its own frozen profile, not an edit of this one.

---

# 97. Transaction Document Format 2

*Added 2026-09-26. Extends §§ 91.3 and 96.4. It adds one transaction-document format version beside
the original; § 91.3's `ekr.transaction-document/1` profile, grammar and refusals are unchanged, and
every retained `/1` proposal is read, validated and replayed exactly as before. Decision record:
`architecture-decision-record:0011-transaction-document-format-2`.*

**Why.** A `/1` document holds at most 256 operations in 262144 bytes, so a change of a few thousand
operations had to be split into many transactions, each proposed, validated and committed on its
own. § 96.4 measured one propose, validate and commit of 10,000 operations at 6.2 s once the store no
longer replayed its history per verb; § 91.3 requires a new version for any changed profile.

## 97.1 The format

`ekr.transaction-document/2` has the envelope, the typed grammar and the refusals of § 91.3; only
`format` differs. It selects this frozen inclusive profile:

| limit | `/2` | `/1` (§ 91.3) |
|---|---|---|
| input bytes | 8388608 (8 MiB) | 262144 |
| container depth | 32 | 32 |
| expanded representation nodes | 1048576 | 32768 |
| entries per map | 4096 | 4096 |
| elements per sequence | 16384 | 4096 |
| decoded bytes per string / per key | 65536 / 4096 | 65536 / 4096 |
| total expanded string bytes | 33554432 | 1048576 |
| operations | 1 through 10000 | 1 through 256 |
| input evidence entries | 10000 | 1024 |

Every `/2` limit is at least its `/1` value, and each keeps `/1`'s ratio to the input cap where it
scales with size (one node per 8 input bytes, four string bytes per input byte). Changing any of them
is a further version. `ekr example`, `ekr schema` and `ekr guide` write `/2`; `ekr schema
ekr.transaction-document/1` still prints the frozen original.

## 97.2 Selecting the profile before parsing

The raw-byte cap must apply before parsing (§ 91.3), so the version is read before the document is.
The reader takes the first line that starts at column 0 with the key `format` (plain or quoted),
removes quotes and a trailing comment, and selects the profile that value names. Without such a line
the `/1` byte cap applies. A document is then parsed under the selected profile and held to the
profile of the version it declares: if the two differ it is parsed again under the declared one, and
a document with no readable `format:` line that `/1` refuses on a limit other than its byte cap is
admitted only if `/2` accepts it and it declares `/2`. So a `/1` document is always held to § 91.3,
and a `/2` document within 262144 bytes is admitted whatever its layout. The bounded reader stops
after the `/1` cap plus one byte unless the bytes read so far name `/2`, and then after the `/2` cap
plus one byte; a stricter host upload cap still applies to new ingress only. A refusal names the
declared version's bound: `transaction document limit: operations (at most 10000 operations per
document; …)`, and a `/1` refusal on operations names `/2`.

## 97.3 What is unchanged, and the evidence

Proposal records keep the exact submitted bytes and `document_hash`; replay reparses them through the
same selection, so a retained proposal's profile cannot change after it is recorded. No canonical
root, validation rule, record format or store format changes. `ekr.proposal-record/1` and `/2` embed
either version's bytes.

Executed by `crates/ekr-kernel/tests/transaction_document_v2.rs`: a `/1` document of 257 operations is
refused with its own bound; a `/2` document of 10,000 operations parses and one of 10,001 is refused
naming 10000; 8388608 bytes are read and 8388609 refused naming 8388608, while a `/1` document over
262144 bytes is refused naming 262144 through both `parse` and `read`; a flow-style document takes
its declared profile under the `/1` byte cap; and
`a_v2_document_of_10000_operations_validates_and_commits_on_both_providers`. The conformance fixture
the generated Propose scenarios submit is a `/2` document, and the unknown-format fixture names `/3`.

**Measured** on the rebuilt store of § 96.4 (70 MB, file provider, release build, median of three
fresh copies): one propose, validate and commit of a `/2` document of 2,000 operations (714 KB) took
1.20 + 1.08 + 1.31 s, 3.60 s in all; of 10,000 operations (3.6 MB), 2.40 + 1.45 + 2.22 s, 6.08 s.
