set local lock_timeout = '2s';
alter table prices
    add column new_price bigint;
