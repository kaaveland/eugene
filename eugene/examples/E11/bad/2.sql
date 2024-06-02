set local lock_timeout = '2s';
alter table prices
    add column id serial;
