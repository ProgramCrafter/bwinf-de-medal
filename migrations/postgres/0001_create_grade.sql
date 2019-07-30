CREATE TABLE grade (
       taskgroup INTEGER,
       session INTEGER,
       grade INTEGER,
       validated BOOL,
       PRIMARY KEY (taskgroup, session)
)
