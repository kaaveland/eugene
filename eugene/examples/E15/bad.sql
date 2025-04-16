-- 1.sql
create table items
(
    id bigint generated always as identity primary key
);

create table purchase
(
    id   bigint generated always as identity primary key,
    item bigint not null references items (id) -- no index
);

-- 2.sql
set local lock_timeout = '2s';
-- eugene: ignore E6
create index purchase_item_idx on purchase (item)
    -- this is a partial index, not good enough for enforcing referential integrity
    where item = 1;

