-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text,
    email text
);

-- 2.sql
set lock_timeout = '2s';
alter table authors
    alter column name set not null;
-- eugene: ignore E2, E4
alter table authors
    alter column email set not null;
