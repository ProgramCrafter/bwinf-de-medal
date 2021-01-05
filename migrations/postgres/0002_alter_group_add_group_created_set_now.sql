ALTER TABLE usergroup ADD COLUMN group_created TIMESTAMP;
UPDATE usergroup SET group_created = NOW() WHERE group_created IS NULL;
