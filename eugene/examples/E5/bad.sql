-- 1.sql
create table prices (
    id integer generated always as identity
        primary key,
    price int not null
);

-- 2.sql
set local lock_timeout = '2s';
alter table prices
    alter price set data type bigint;
