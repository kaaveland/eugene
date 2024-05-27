set local lock_timeout = '2s';
-- eugene trace knows name has a valid not null check, but eugene lint doesn't
-- eugene: ignore E2
alter table authors
    alter name set not null;
