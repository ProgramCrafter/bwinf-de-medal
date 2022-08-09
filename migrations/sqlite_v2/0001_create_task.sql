CREATE TABLE task (
       id INTEGER PRIMARY KEY,
       taskgroup INTEGER REFERENCES taskgroup (id) ON DELETE CASCADE,
       location TEXT,
       stars INTEGER
)
