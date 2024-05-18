set local lock_timeout = '2s';
alter table authors validate constraint check_name_not_null;
