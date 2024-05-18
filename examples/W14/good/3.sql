set local lock_timeout = '2s';
-- eugene: ignore E2
-- This is a demo of W14, so we can ignore E2 instead of the
-- multi-step migration to make the column NOT NULL safely
alter table authors alter column name set not null;
