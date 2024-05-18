set local lock_timeout = '2s';
alter table prices validate constraint check_new_price_not_null, drop column price;
-- eugene: ignore E4
-- this has to run in the same transaction as dropping the old price column
alter table prices rename column new_price to price;
