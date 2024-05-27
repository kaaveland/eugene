create table document_type(
    type_name text primary key
);
insert into document_type
  values('invoice'), ('receipt'), ('other');
create table document (
    id int generated always as identity
        primary key,
    type text
        references document_type(type_name)
);
