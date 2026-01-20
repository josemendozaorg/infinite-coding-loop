# Logic: 19_grpc_remote_workers

## User Logic
- Connect external worker nodes (e.g., Python runners) to the core loop.
- Scale horizontally across machines.

## Technical Logic
- `tonic` gRPC server.
- Shared `.proto` definition for `WorkerService`.
- Bi-directional streaming for task updates.

## Implementation Strategy
1. Define `worker.proto`.
2. Generate Rust code with `prost`.
3. Implement `GrpcOrchestrator` client and server.
