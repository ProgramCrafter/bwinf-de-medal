CREATE TABLE session (
       id SERIAL PRIMARY KEY,
       session_token TEXT,
       csrf_token TEXT,
       last_login TIMESTAMP,
       last_activity TIMESTAMP,
       permanent_login BOOL,

       username TEXT,
       password TEXT,
       salt TEXT,
       logincode TEXT,
       email TEXT,
       email_unconfirmed TEXT,
       email_confirmationcode TEXT,

       firstname TEXT,
       lastname TEXT,
       street TEXT,
       zip TEXT,
       city TEXT,
       nation TEXT,
       grade INTEGER,

       is_teacher BOOL,
       managed_by INTEGER,
       oauth_foreign_id TEXT,
       oauth_provider TEXT
)
