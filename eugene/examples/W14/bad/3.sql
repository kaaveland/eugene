set local lock_timeout = '2s';
alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key;
