CREATE TABLE app_lock (
  id      INTEGER PRIMARY KEY CHECK (id = 1),
  salt    TEXT    NOT NULL,
  hash    TEXT    NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0
);