CREATE TABLE taskgroup (
       id INTEGER PRIMARY KEY,
       contest INTEGER NOT NULL REFERENCES contest (id) ON DELETE CASCADE,
       name TEXT NOT NULL
)
