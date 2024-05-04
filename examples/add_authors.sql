create table authors(name text not null);
alter table books alter column title set not null;
select pg_sleep(1);
select * from books;

