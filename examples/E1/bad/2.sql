set local lock_timeout = '2s';
alter table authors add constraint name_not_null check (name is not null);
