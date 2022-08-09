CREATE TABLE grade (
       taskgroup INTEGER REFERENCES taskgroup (id) ON DELETE CASCADE,
       session INTEGER REFERENCES session (id) ON DELETE CASCADE,
       grade INTEGER,
       validated INTEGER,
       PRIMARY KEY (taskgroup, session)
)
