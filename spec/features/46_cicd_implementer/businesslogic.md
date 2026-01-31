# Logic: 46_cicd_implementer

## Core Logic

### 1. Workflow Generation
- Generate `.github/workflows/ci.yml`.
- Include steps for: Checkout, Cache, Lint, Build, Test.

### 2. Maintenance
- Update workflow if toolchain changes (e.g., node version update).

## Data Flow
ProjectInit -> CICDImplementer -> WorkflowFile
