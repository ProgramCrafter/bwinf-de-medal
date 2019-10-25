CREATE TABLE session (
       id INTEGER PRIMARY KEY,
       session_token TEXT,
       csrf_token TEXT,
       last_login TEXT,
       last_activity TEXT,
       permanent_login INTEGER,

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

       is_teacher INTEGER,
       managed_by INTEGER,
       oauth_foreign_id TEXT,
       oauth_provider TEXT
)
