-- 1.sql
create table authors(
    name text
);

-- 2.sql
create unique index concurrently
    authors_name_key on authors(name);

-- 3.sql
set local lock_timeout = '2s';
alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key;
