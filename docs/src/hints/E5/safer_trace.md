## Eugene 🔒 trace report of `examples/E5/good/1.sql`



### Statement number 1 for 10ms

#### SQL

```sql
create table prices (id integer generated always as identity primary key, price int not null)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E5/good/2.sql`



### Statement number 1 for 10ms

#### SQL

```sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



### Statement number 2 for 10ms

#### SQL

```sql
alter table prices add column new_price bigint
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `prices` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |



## Eugene 🔒 trace report of `examples/E5/good/3.sql`



### Statement number 1 for 10ms

#### SQL

```sql
update prices set new_price = price :: bigint
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



### Statement number 2 for 10ms

#### SQL

```sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



### Statement number 3 for 10ms

#### SQL

```sql
alter table prices add constraint check_new_price_not_null check (new_price is not null) not valid
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `prices` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |



## Eugene 🔒 trace report of `examples/E5/good/4.sql`



### Statement number 1 for 10ms

#### SQL

```sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



### Statement number 2 for 10ms

#### SQL

```sql
alter table prices validate constraint check_new_price_not_null, drop column price
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `prices` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |



### Statement number 3 for 10ms

#### SQL

```sql
-- eugene: ignore E4
-- this has to run in the same transaction as dropping the old price column
alter table prices rename column new_price to price
```

#### Locks at start

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `prices` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |

#### New locks taken

No new locks taken by this statement.


