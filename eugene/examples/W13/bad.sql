-- 1.sql
create type document_type
    as enum ('invoice', 'receipt', 'other');
create table document (
    id int generated always as identity
        primary key,
    type document_type
);
