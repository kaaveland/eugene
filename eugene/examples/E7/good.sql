-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql
create unique index concurrently
    authors_name_unique on authors(name);

-- 3.sql
set local lock_timeout = '2s';
alter table authors
    add constraint unique_name
        unique using index authors_name_unique;
