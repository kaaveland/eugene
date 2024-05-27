set local lock_timeout = '2s';
alter table authors
    add column meta jsonb;
