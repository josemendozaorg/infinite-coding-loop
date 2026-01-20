#!/bin/bash
# verify_traces.sh - Dump events grouped by trace_id to see causality chains

DB_FILE="ifcl.db"

if [ ! -f "$DB_FILE" ]; then
    echo "Error: $DB_FILE not found. Run the TUI first to generate data."
    exit 1
fi

echo "--- CAUSALITY CHAINS (Grouped by Trace ID) ---"
sqlite3 "$DB_FILE" <<EOF
.mode column
.headers on
SELECT 
    substr(trace_id, 1, 8) as trace_short,
    event_type,
    worker_id,
    substr(payload, 1, 40) as preview
FROM events 
ORDER BY trace_id, timestamp;
EOF
