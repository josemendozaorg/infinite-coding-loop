CREATE TABLE IF NOT EXISTS applications (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    work_dir TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS primitives (
    id TEXT NOT NULL,
    app_id TEXT NOT NULL,
    type TEXT NOT NULL,
    payload BLOB NOT NULL,
    version INTEGER DEFAULT 1,
    PRIMARY KEY (id, app_id),
    FOREIGN KEY (app_id) REFERENCES applications(id)
);
