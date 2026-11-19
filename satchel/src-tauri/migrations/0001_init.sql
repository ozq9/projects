-- Satchel schema v1.
-- Timestamps are ISO-8601 UTC strings ("2026-11-19T20:00:00.000Z") so they sort lexicographically.
-- The sqlite-vec virtual table for embeddings arrives in a later migration (milestone 5), once the
-- embedding model (and therefore the vector dimension) is fixed.

CREATE TABLE subjects (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL COLLATE NOCASE UNIQUE,
    color       TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE items (
    id              INTEGER PRIMARY KEY,
    subject_id      INTEGER REFERENCES subjects(id) ON DELETE SET NULL,
    title           TEXT,
    item_type       TEXT CHECK (item_type IN ('worksheet', 'lecture_material', 'notes', 'table_data', 'other')),
    source_kind     TEXT    NOT NULL,          -- image | pdf | pptx | docx | text | clipboard_image | webcam
    original_name   TEXT,                      -- file name as dropped, for display
    source_path     TEXT,                      -- path of the stored copy, relative to the data folder
    mime_type       TEXT,
    content_hash    TEXT,                      -- sha-256 of the original, used to spot duplicates
    extracted_text  TEXT,
    ocr_confidence  REAL,                      -- 0..1 quality score (see README: Windows OCR has no native confidence)
    status          TEXT    NOT NULL DEFAULT 'queued',
    error           TEXT,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX items_by_subject ON items(subject_id, created_at DESC);
CREATE INDEX items_by_created ON items(created_at DESC);
CREATE INDEX items_by_status  ON items(status);
CREATE INDEX items_by_hash    ON items(content_hash);

CREATE TABLE result_cards (
    id            INTEGER PRIMARY KEY,
    item_id       INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    kind          TEXT    NOT NULL,            -- worksheet_solutions | summary | cleaned_notes | table
    content_json  TEXT    NOT NULL,
    model         TEXT,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX result_cards_by_item ON result_cards(item_id);

CREATE TABLE flashcards (
    id          INTEGER PRIMARY KEY,
    item_id     INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    front       TEXT    NOT NULL,
    back        TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX flashcards_by_item ON flashcards(item_id);

CREATE TABLE todos (
    id           INTEGER PRIMARY KEY,
    item_id      INTEGER REFERENCES items(id) ON DELETE SET NULL,
    subject_id   INTEGER REFERENCES subjects(id) ON DELETE SET NULL,
    description  TEXT    NOT NULL,
    due_date     TEXT,                         -- ISO date (YYYY-MM-DD) or datetime
    done         INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX todos_open_by_due ON todos(done, due_date);
CREATE INDEX todos_by_subject  ON todos(subject_id);

CREATE TABLE chunks (
    id       INTEGER PRIMARY KEY,
    item_id  INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    seq      INTEGER NOT NULL,                 -- order of the chunk within the item
    text     TEXT    NOT NULL
);
CREATE INDEX chunks_by_item ON chunks(item_id, seq);

CREATE TABLE chat_messages (
    id              INTEGER PRIMARY KEY,
    subject_id      INTEGER REFERENCES subjects(id) ON DELETE CASCADE,  -- NULL = library-wide chat
    role            TEXT    NOT NULL CHECK (role IN ('user', 'assistant')),
    content         TEXT    NOT NULL,
    cited_item_ids  TEXT    NOT NULL DEFAULT '[]',                       -- JSON array of item ids
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX chat_by_subject ON chat_messages(subject_id, created_at);

CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);

-- One row per Claude API call, for the token/cost counter in Settings.
CREATE TABLE ai_usage (
    id                     INTEGER PRIMARY KEY,
    created_at             TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    purpose                TEXT    NOT NULL,   -- connection_test | classify | result_card | chat | vision_ocr ...
    model                  TEXT    NOT NULL,   -- the model that actually served the call
    item_id                INTEGER REFERENCES items(id) ON DELETE SET NULL,
    input_tokens           INTEGER NOT NULL DEFAULT 0,
    output_tokens          INTEGER NOT NULL DEFAULT 0,
    cache_creation_tokens  INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens      INTEGER NOT NULL DEFAULT 0,
    cost_usd               REAL                -- NULL when the model's price is unknown
);
CREATE INDEX ai_usage_by_created ON ai_usage(created_at);
