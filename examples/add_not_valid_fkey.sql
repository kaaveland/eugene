create table authors(id serial primary key, name text);
alter table books add column author_id integer null;
alter table books add constraint fk_books_authors foreign key (author_id) references authors(id) not valid;
