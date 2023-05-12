CREATE TABLE participation (
       contest INTEGER REFERENCES contest (id) ON DELETE CASCADE,
       session INTEGER REFERENCES session (id) ON DELETE CASCADE,
       start_date TEXT,
       PRIMARY KEY (contest, session)
)
