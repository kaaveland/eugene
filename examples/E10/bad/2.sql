set local lock_timeout = '2s';
alter table authors add column meta jsonb;

-- eugene: ignore E5, E4
-- causes table rewrite, but this example isnt't about that
alter table prices alter price set data type bigint;
