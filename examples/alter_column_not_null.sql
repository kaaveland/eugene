alter table books alter column title set not null;
alter table books add constraint title_unique unique (title);
