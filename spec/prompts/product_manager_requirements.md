Extract atomic requirements from this user request: '{{input}}'.
Output purely YAML format list of Requirement structs.
Each struct MUST have exact fields:
- id: string
- user_story: string
- acceptance_criteria: list of strings
- kind: 'Requirement'
