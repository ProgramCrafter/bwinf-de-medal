UPDATE session SET account_created = NOW() WHERE account_created IS NULL;
