-- 1.sql
create table prices (
    id integer generated always as identity
        primary key,
    price int not null
);

create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql
set local lock_timeout = '2s';
alter table authors
    add column meta jsonb;

-- 3.sql
set local lock_timeout = '2s';
-- eugene: ignore E5, E4
-- causes table rewrite, but this example isnt't about that
alter table prices
    alter price set data type bigint;
