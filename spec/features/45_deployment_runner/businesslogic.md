# Logic: 45_deployment_runner

## Core Logic

### 1. Local Deployment
- `cargo run`.
- Docker Compose up.

### 2. Remote Deployment
- SSH / SCP.
- Cloud/Edge worker deploy (e.g., Shuttle, Fly.io if supported).

## Data Flow
StableBuild -> DeployRunner -> URL/Status
