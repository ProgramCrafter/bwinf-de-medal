CREATE TABLE contest (
       id INTEGER PRIMARY KEY,
       location TEXT NOT NULL,
       filename TEXT NOT NULL,
       name TEXT NOT NULL,
       duration INTEGER NOT NULL,
       public INTEGER NOT NULL,
       start_date TEXT,
       end_date TEXT
)

