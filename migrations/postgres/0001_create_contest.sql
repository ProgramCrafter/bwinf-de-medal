CREATE TABLE contest (
       id SERIAL PRIMARY KEY,
       location TEXT NOT NULL,
       filename TEXT NOT NULL,
       name TEXT NOT NULL,
       duration INTEGER NOT NULL,
       public BOOL NOT NULL,
       start_date TIMESTAMP,
       end_date TIMESTAMP
)

