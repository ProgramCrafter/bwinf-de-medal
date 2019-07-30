CREATE TABLE submission (
       id SERIAL PRIMARY KEY,
       session INTEGER NOT NULL,
       task INTEGER NOT NULL,
       grade INTEGER NOT NULL,
       validated BOOL NOT NULL,
       needs_validation BOOL NOT NULL,
       nonvalidated_grade INTEGER NOT NULL,
       subtask_identifier TEXT,
       value TEXT,
       date TIMESTAMP
)
