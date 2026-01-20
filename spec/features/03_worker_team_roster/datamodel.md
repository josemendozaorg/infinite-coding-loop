# Data Model: 03_worker_team_roster

## Entities
The data structures required to define the "Team" and their "Relationships".

### Worker Group Profile
Defined in `group.yaml`.

```rust
pub struct WorkerGroupProfile {
    pub name: String,
    pub description: String,
    pub workers: Vec<WorkerDef>,
}

pub struct WorkerDef {
    pub id: String, // "git-01", "claude-arch"
    pub role:  WorkerRole, // Enum: Git, Researcher, Coder...
    pub config: WorkerConfig, // { model: "claude-3-5", temp: 0.2 }
}
```

### Relationship Profile (Topology)
Defined in `topology.yaml`. This defines how workers are allowed to communicate.

```rust
pub struct RelationshipProfile {
    pub name: String,
    pub edges: Vec<RelationshipEdge>
}

pub struct RelationshipEdge {
    pub from_role: WorkerRole,
    pub to_role: WorkerRole,
    pub interaction_type: InteractionType, // PeerReview, Command, Query
    pub protocol: String // "gRPC/v1"
}
```

## Storage
- These are **Static Configurations** loaded at startup.
- They are also stamped into the `EventLog` via the `LoopStarted` event so we have a permanent record of *who* performed the work.