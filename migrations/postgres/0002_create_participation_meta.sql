CREATE TABLE participation_meta (
       contest INTEGER REFERENCES contest(id) ON DELETE CASCADE,
       session INTEGER REFERENCES session(id) ON DELETE CASCADE,
       jwinf_round3_admission BOOL,
);

CREATE INDEX ON participation_meta (session, contest);
