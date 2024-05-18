set local lock_timeout = '2s';
alter table authors add constraint check_name_not_null check (name is not null) not valid;
