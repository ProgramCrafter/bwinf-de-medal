CREATE INDEX ON grade (session);
CREATE INDEX ON grade (session, taskgroup);

CREATE INDEX ON participation (session);
CREATE INDEX ON participation (session, contest);

CREATE INDEX ON taskgroup (contest);

CREATE INDEX ON task (taskgroup);

CREATE INDEX ON usergroup (admin);

CREATE INDEX ON session (managed_by);
