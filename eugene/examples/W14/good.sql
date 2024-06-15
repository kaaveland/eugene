-- 1.sql
create table authors(
    name text
);

-- 2.sql
create unique index concurrently
    authors_name_key on authors(name);

-- 3.sql
set local lock_timeout = '2s';
-- eugene: ignore E2
-- This is a demo of W14, so we can ignore E2 instead of the
-- multi-step migration to make the column NOT NULL safely
alter table authors
    alter column name set not null;

-- 4.sql
alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key;
