-- 1.sql
create table prices (
    price int not null
);

-- 2.sql
set local lock_timeout = '2s';
alter table prices
    add column id serial;
