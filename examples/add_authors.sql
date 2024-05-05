create table authors(id serial primary key, name text not null);
alter table books alter column title set not null;
alter table books add column author_id integer not null;
alter table books add foreign key (author_id) references authors(id);
select pg_sleep(1);
select * from books;

