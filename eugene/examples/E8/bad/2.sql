set local lock_timeout = '2s';
alter table authors
    add constraint authors_name_excl
        exclude (name with =);
