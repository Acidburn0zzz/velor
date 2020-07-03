CREATE TABLE IF NOT EXISTS "data_migration" (
    id INTEGER PRIMARY KEY NOT NULL,
    title VARCHAR(64) NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    last_run TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);