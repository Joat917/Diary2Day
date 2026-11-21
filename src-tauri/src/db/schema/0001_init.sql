CREATE TABLE journal (
  id           TEXT    NOT NULL PRIMARY KEY,
  content      TEXT    NOT NULL DEFAULT '',
  format       TEXT    NOT NULL DEFAULT 'markdown' CHECK (format IN ('markdown','plain')),
  title        TEXT,
  mood         TEXT,
  recorded_at  TEXT    NOT NULL,
  recorded_off INTEGER NOT NULL DEFAULT 0,
  day_key      TEXT    NOT NULL,
  created_at   TEXT    NOT NULL,
  updated_at   TEXT    NOT NULL,
  deleted_at   TEXT,
  pinned       INTEGER NOT NULL DEFAULT 0,
  sensitive    INTEGER NOT NULL DEFAULT 0,
  words        INTEGER NOT NULL DEFAULT 0,
  asset_n      INTEGER NOT NULL DEFAULT 0,
  cover_asset  TEXT,
  revision     INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_j_day      ON journal(day_key, recorded_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_j_recorded ON journal(recorded_at DESC)          WHERE deleted_at IS NULL;
CREATE INDEX idx_j_trash    ON journal(deleted_at DESC)           WHERE deleted_at IS NOT NULL;

CREATE TABLE tag (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  name_key   TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL
);

CREATE TABLE journal_tag (
  journal_id TEXT NOT NULL REFERENCES journal(id) ON DELETE CASCADE,
  tag_id     TEXT NOT NULL REFERENCES tag(id)     ON DELETE CASCADE,
  PRIMARY KEY (journal_id, tag_id)
);
CREATE INDEX idx_jt_tag ON journal_tag(tag_id);

CREATE TABLE day_mark (
  day_key TEXT PRIMARY KEY,
  n       INTEGER NOT NULL DEFAULT 0,
  words   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE asset (
  name        TEXT PRIMARY KEY,
  kind        TEXT NOT NULL CHECK (kind IN ('image','audio','video')),
  ext         TEXT NOT NULL,
  size        INTEGER NOT NULL,
  width       INTEGER,
  height      INTEGER,
  duration_ms INTEGER,
  hash        BLOB,
  orig_name   TEXT,
  created_at  TEXT NOT NULL
);

CREATE TABLE settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE VIRTUAL TABLE journal_fts USING fts5(
  title,
  content,
  content='journal',
  content_rowid='rowid',
  tokenize='trigram case_sensitive 0'
);

CREATE TRIGGER journal_ai AFTER INSERT ON journal BEGIN
  INSERT INTO journal_fts(rowid, title, content)
  VALUES (new.rowid, coalesce(new.title, ''), new.content);
END;

CREATE TRIGGER journal_ad AFTER DELETE ON journal BEGIN
  INSERT INTO journal_fts(journal_fts, rowid, title, content)
  VALUES ('delete', old.rowid, coalesce(old.title, ''), old.content);
END;

CREATE TRIGGER journal_au AFTER UPDATE OF title, content ON journal BEGIN
  INSERT INTO journal_fts(journal_fts, rowid, title, content)
  VALUES ('delete', old.rowid, coalesce(old.title, ''), old.content);
  INSERT INTO journal_fts(rowid, title, content)
  VALUES (new.rowid, coalesce(new.title, ''), new.content);
END;