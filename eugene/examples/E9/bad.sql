-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql
alter table authors add column email text;
