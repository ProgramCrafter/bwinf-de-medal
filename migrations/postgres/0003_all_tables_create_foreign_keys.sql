ALTER TABLE grade ADD CONSTRAINT grade_session_fkey FOREIGN KEY (session) REFERENCES session (id) ON DELETE CASCADE;
ALTER TABLE grade ADD CONSTRAINT grade_taskgroup_fkey FOREIGN KEY (taskgroup) REFERENCES taskgroup (id) ON DELETE CASCADE;
ALTER TABLE participation ADD CONSTRAINT participation_session_fkey FOREIGN KEY (session) REFERENCES session (id) ON DELETE CASCADE;
ALTER TABLE submission ADD CONSTRAINT submission_session_fkey FOREIGN KEY (session) REFERENCES session (id) ON DELETE CASCADE;
ALTER TABLE submission ADD CONSTRAINT submission_task_fkey FOREIGN KEY (task) REFERENCES task (id) ON DELETE CASCADE;
ALTER TABLE task ADD CONSTRAINT task_taskgroup_fkey FOREIGN KEY (taskgroup) REFERENCES taskgroup (id) ON DELETE CASCADE;
ALTER TABLE taskgroup ADD CONSTRAINT taskgroup_contest_fkey FOREIGN KEY (contest) REFERENCES contest (id) ON DELETE CASCADE;
ALTER TABLE usergroup ADD CONSTRAINT usergroup_admin_fkey FOREIGN KEY (admin) REFERENCES session (id) ON DELETE CASCADE;
ALTER TABLE session ADD CONSTRAINT session_managed_by_fkey FOREIGN KEY (managed_by) REFERENCES usergroup (id) ON DELETE RESTRICT;
