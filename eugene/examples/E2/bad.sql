-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text
);

-- 2.sql
set local lock_timeout = '2s';
alter table authors
    alter column name set not null;
