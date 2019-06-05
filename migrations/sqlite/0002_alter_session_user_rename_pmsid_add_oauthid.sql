ALTER TABLE session_user ADD COLUMN oauth_foreign_id TEXT;
ALTER TABLE session_user ADD COLUMN oauth_provider TEXT;
UPDATE session_user SET (oauth_foreign_id, oauth_provider) = (pms_id, "pms") WHERE pms_id IS NOT NULL;
