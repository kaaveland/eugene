alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key;
