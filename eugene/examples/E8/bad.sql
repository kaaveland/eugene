-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql
set local lock_timeout = '2s';
alter table authors
    add constraint authors_name_excl
        exclude (name with =);
