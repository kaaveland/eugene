set local lock_timeout = '2s';
alter table authors
    add constraint unique_name unique(name);
