-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null,
    meta json
);
