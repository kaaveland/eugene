create table authors(id serial primary key, name text);
alter table books add column author_id integer null;
-- eugene: ignore W12
-- for this example, we're targeting the table twice to show the difference in locks
-- and we can't do that if we add the constraint as valid to only alter the table on
alter table books add constraint fk_books_authors foreign key (author_id) references authors(id) not valid;
