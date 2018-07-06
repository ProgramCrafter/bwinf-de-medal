CREATE TABLE submission (
       id INTEGER PRIMARY KEY,
       session_user INTEGER NOT NULL,
       task INTEGER NOT NULL,
       grade INTEGER NOT NULL,
       validated INTEGER NOT NULL,
       nonvalidated_grade INTEGER NOT NULL,
       subtask_identifier TEXT,
       value TEXT,
       date TEXT
)
