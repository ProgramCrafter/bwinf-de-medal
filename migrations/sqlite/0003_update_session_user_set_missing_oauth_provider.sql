UPDATE session_user SET oauth_provider = "pms" WHERE oauth_foreign_id IS NOT NULL;
