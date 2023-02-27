ALTER TABLE participation ADD CONSTRAINT participation_contest_fkey FOREIGN KEY (contest) REFERENCES contest (id) ON DELETE CASCADE;
