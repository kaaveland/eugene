-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text
);

-- 2.sql
set local lock_timeout = '2s';
alter table authors
    add constraint check_name_not_null
        check (name is not null) not valid;

-- 3.sql
set local lock_timeout = '2s';
alter table authors
    validate constraint check_name_not_null;

-- 4.sql
set local lock_timeout = '2s';
-- eugene trace knows name has a valid not null check, but eugene lint doesn't
-- eugene: ignore E2
alter table authors
    alter name set not null;
