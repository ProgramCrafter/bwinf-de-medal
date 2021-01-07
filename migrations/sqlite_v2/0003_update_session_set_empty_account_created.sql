UPDATE session SET account_created = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE account_created IS NULL;
