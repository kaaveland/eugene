set lock_timeout = '2s';
alter table authors
    alter column name set not null;
-- eugene: ignore E2, E4
alter table authors
    alter column email set not null;
