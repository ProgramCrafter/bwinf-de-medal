CREATE TABLE submission (
       id INTEGER PRIMARY KEY,
       session INTEGER NOT NULL REFERENCES session (id) ON DELETE CASCADE,
       task INTEGER NOT NULL REFERENCES task (id) ON DELETE CASCADE,
       grade INTEGER NOT NULL,
       validated INTEGER NOT NULL,
       needs_validation INTEGER NOT NULL,
       nonvalidated_grade INTEGER NOT NULL,
       subtask_identifier TEXT,
       value TEXT,
       date TEXT
)
