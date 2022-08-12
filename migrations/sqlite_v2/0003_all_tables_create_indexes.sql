CREATE INDEX grade_session_idx ON grade (session);
CREATE INDEX grade_session_taskgroup_idx ON grade (session, taskgroup);

CREATE INDEX participation_session_idx ON participation (session);
CREATE INDEX participation_session_contest_idx ON participation (session, contest);

CREATE INDEX taskgroup_contest_idx ON taskgroup (contest);

CREATE INDEX task_taskgroup_idx ON task (taskgroup);

CREATE INDEX usergroup_admin_idx ON usergroup (admin);

CREATE INDEX session_managed_by_idx ON session (managed_by);
