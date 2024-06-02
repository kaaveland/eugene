set lock_timeout = '2s';
-- eugene: ignore E2
alter table authors
  alter column name set not null,
  alter column email set not null;
