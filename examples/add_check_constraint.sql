alter table books add constraint check_title_not_null check (title is not null) not valid;
alter table books validate constraint check_title_not_null; -- this takes a different, lesser lock
