CREATE TABLE submission (
       id INTEGER PRIMARY KEY,
       session INTEGER NOT NULL,
       task INTEGER NOT NULL,
       grade INTEGER NOT NULL,
       validated INTEGER NOT NULL,
       needs_validation INTEGER NOT NULL,
       nonvalidated_grade INTEGER NOT NULL,
       subtask_identifier TEXT,
       value TEXT,
       date TEXT
)
