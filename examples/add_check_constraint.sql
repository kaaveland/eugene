alter table books add constraint check_title_not_null check (title is not null) not valid;
-- eugene: ignore W12
-- for this example, we're targeting the table twice to show the difference in locks
-- and we can't do that if we add the constraint as valid to only alter the table once
alter table books validate constraint check_title_not_null; -- this takes a different, lesser lock
