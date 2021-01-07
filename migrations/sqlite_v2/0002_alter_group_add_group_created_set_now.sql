ALTER TABLE usergroup ADD COLUMN group_created TEXT;
UPDATE usergroup SET group_created = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE group_created IS NULL;
