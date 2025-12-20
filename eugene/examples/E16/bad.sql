-- 1.sql
create table products (
    id integer generated always as identity primary key,
    name text not null,
    price decimal(10,2) not null
);

-- 2.sql
set local lock_timeout = '2s';

create or replace function products_trigger_fn()
    returns trigger as
$$
begin
    -- Simple trigger that logs changes
    raise notice 'Product % was modified', new.id;
    return new;
end;
$$ language plpgsql;

create trigger products_audit_trigger
    after insert or update
    on products
    for each row
execute function products_trigger_fn();

select count(*) from products where name = 'Widget';