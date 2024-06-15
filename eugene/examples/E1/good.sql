-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text
);

-- 2.sql
set local lock_timeout = '2s';
alter table authors
    add constraint name_not_null
        check (name is not null) not valid;

-- 3.sql
set local lock_timeout = '2s';
alter table authors
    validate constraint name_not_null;
