# Logic: 03_worker_team_roster

## Core Logic

### 1. Worker Manager
- **Registry**: HashMap of `WorkerID` -> `WorkerHandle` (ActorRef).
- **Lifecycle**: `spawn_worker()`, `terminate_worker()`, `restart_worker()`.
- **Health Check**: Periodically ping workers to ensure they are responsive.

### 2. Team Profiles
- **Loader**: Load team configurations from JSON/TOML.
- **Dynamic Scaling**: Logic to add more "Coders" if the queue is backing up.

## Data Flow
User/Auto-Scalar -> WorkerManager -> Spawn Actor -> Update Roster UI