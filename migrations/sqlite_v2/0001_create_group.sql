CREATE TABLE usergroup (
       id INTEGER PRIMARY KEY,
       name TEXT,
       groupcode TEXT,
       tag TEXT,
       admin INTEGER REFERENCES session (id) ON DELETE CASCADE
)
