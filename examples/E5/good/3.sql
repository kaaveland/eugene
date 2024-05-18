update prices set new_price = price :: bigint;
set local lock_timeout = '2s';
alter table prices add constraint check_new_price_not_null check (new_price is not null) not valid;
