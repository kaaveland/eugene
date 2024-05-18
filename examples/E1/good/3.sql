set local lock_timeout = '2s';
alter table authors validate constraint name_not_null;
