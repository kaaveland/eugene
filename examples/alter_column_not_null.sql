alter table books alter column title set not null, add constraint title_unique unique (title);
