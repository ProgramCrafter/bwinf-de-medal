CREATE TABLE participation_meta (
       contest INTEGER REFERENCES contest (id) ON DELETE CASCADE,
       session INTEGER REFERENCES session (id) ON DELETE CASCADE,
       jwinf_round3_admission INTEGER,
);

CREATE INDEX participation_meta_session_idx ON participation_meta (session);
CREATE INDEX participation_meta_session_contest_idx ON participation_meta (session, contest);
