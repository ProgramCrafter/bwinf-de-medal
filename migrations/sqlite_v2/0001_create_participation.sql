CREATE TABLE participation (
       contest INTEGER,
       session INTEGER REFERENCES session (id) ON DELETE CASCADE,
       start_date TEXT,
       PRIMARY KEY (contest, session)
)
