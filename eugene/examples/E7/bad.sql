-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql
set local lock_timeout = '2s';
alter table authors
    add constraint unique_name unique(name);
