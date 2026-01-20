# Comprehensive Data Model Specification

## 1. Core Philosophy: Event Sourcing
The system state is a function of its history. `State = f(Events)`.
Data is strictly strictly typed and immutable once written.

## 2. The Event Log (Source of Truth)
**Storage:** SQLite (local strategy) or Postgres (remote strategy).
**Schema:** `events` table.

```sql
CREATE TABLE events (
    event_id        UUID PRIMARY KEY,
    sequence_id     INTEGER AUTOINCREMENT, -- Global ordering
    timestamp_utc   TEXT NOT NULL,         -- ISO8601
    worker_id       TEXT NOT NULL,         -- "actor"
    trace_id        UUID NOT NULL,         -- "causality chain"
    event_type      TEXT NOT NULL,         -- Discriminator
    version         INTEGER DEFAULT 1,     -- Schema version
    payload         BLOB NOT NULL          -- Protobuf/JSON
);

CREATE INDEX idx_events_timestamp ON events(timestamp_utc);
CREATE INDEX idx_events_worker ON events(worker_id);
CREATE INDEX idx_events_trace ON events(trace_id);
```

### 2.1. Event Types (The Taxonomy)
Every event payload maps to a Rust Struct / Protobuf Message.

#### Lifecycle Events
- `SystemStarted`: { version: String, config_hash: String }
- `SystemPaused`: { reason: String, paused_by: UserID }
- `SystemResumed`: { timestamp: String }
- `SnapshotCreated`: { snapshot_path: String, last_event_id: UUID }

#### Orchestration Events
- `QuestCreated`: { quest_id: UUID, goal: String, constraints: Map<String,Val> }
- `QuestStatusChanged`: { quest_id: UUID, status: Enum(Pending, Active, Completed, Failed) }
- `WorkerRegistered`: { worker_id: String, profile_data: JSON }
- `TaskAssigned`: { task_id: UUID, worker_id: String, inputs: JSON }

#### Worker Execution Events
- `ThoughtEmitted`: { worker_id: String, thought: String, confidence: Float }
- `ToolUsed`: { worker_id: String, tool_name: String, args: JSON }
- `ToolResult`: { worker_id: String, result: JSON, is_error: Bool }
- `ArtifactGenerated`: { path: String, diff: String, hash: String }
- `CodeCommitted`: { commit_hash: String, message: String }

#### Gamification & Social Events
- `CoinEarned`: { worker_id: String, amount: Int, reason: String }
- `ResourceConsumed`: { worker_id: String, cpu_co2: Float, token_cost: Float }
- `SocraticDialogue`: { question: String, answer: String, consensus: Bool }

## 3. Configuration Profiles (Portable Data)
Serialized as YAML.

### 3.1. Worker Group Profile (`models::profiles::WorkerGroup`)
```rust
struct WorkerGroupProfile {
    name: String,
    version: String,
    workers: Vec<WorkerDef>
}

struct WorkerDef {
    id: String,         // "claude-architect"
    kind: WorkerKind,   // Enum: Native, LLM, Remote
    model: String,      // "claude-3-5-sonnet"
    role: String,       // "Architect"
    capabilities: Vec<String>,
    cost_budget: f64
}
```

### 3.2. Relationship Profile (`models::profiles::Topology`)
```rust
struct Topology {
    name: String,
    channels: Vec<ChannelDef>
}

struct ChannelDef {
    source: String,     // worker_id
    target: String,     // worker_id
    kind: ChannelType,  // OneWay, RequestResponse, PubSub
    filter: String      // "only errors" or "all"
}
```

## 4. Runtime Game State (In-Memory Projection)
This state is rebuilt from the Event Log on startup.
**Struct:** `models::state::global::WorldState`

```rust
struct WorldState {
    tick: u64,
    bank: BankState,
    quests: HashMap<Uuid, QuestState>,
    workers: HashMap<String, WorkerRuntimeState>,
    artifacts: FileSystemIndex
}

struct BankState {
    total_coins: u64,
    total_spend: f64, // Real money
    history: Vec<Transaction>
}

struct WorkerRuntimeState {
    status: WorkerStatus, // Idle, Busy, Paused, Error
    current_task: Option<Uuid>,
    xp_level: u32,
    efficiency_score: f32
}
```

## 5. Protocol Messages (Inter-Worker Communication)
Defined in Protobuf for gRPC.

```protobuf
syntax = "proto3";

message ProtocolMessage {
    string id = 1;
    string trace_id = 2;
    string source_worker = 3;
    string target_worker = 4;
    oneof payload {
        Assignment assignment = 10;
        Report report = 11;
        Correction correction = 12;
    }
}

message Assignment {
    string instructions = 1;
    map<string, string> context = 2;
}
```
