ALTER TABLE contest ADD COLUMN protected BOOL;
UPDATE contest SET protected = false WHERE protected IS NULL;
ALTER TABLE contest ALTER COLUMN protected SET NOT NULL;
