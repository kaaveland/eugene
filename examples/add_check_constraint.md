# Eugene 🔒 trace report of `add_check_constraint.sql`

This is a human readable lock tracing and migration report generated by [eugene](https://github.com/kaaveland/eugene) to assist you in writing safer database migration scripts.

Here are some tips for reading it:
- A lock is called **dangerous** ❌ if it will cause concurrent queries to **wait** for the migration to complete
- You read that right, once a lock is acquired, it is only released at the end of the script
- Eugene will tell you what kinds of queries **dangerous** locks would block in a summary
- **Hints** can sometimes help you avoid dangerous locks, or hold them for a shorter time
- It is hard to avoid dangerous locks, but we should minimize time spent while holding them
- Sometimes seemingly fast migration scripts cause long outages because of lock queues, [here is an example scenario](https://kaveland.no/careful-with-that-lock-eugene.html)

There is a summary section for the entire script at the start of the report and then a section for each statement in the script, that goes over the state of the database at the time the script was executed, as well as effects or hints specific to that particular statement

## Overall Summary

Started at | Total duration (ms) | Number of dangerous locks
---------- | ------------------- | -------------------------
2021-01-01T01:00:00+01:00 | 20 | 1 ❌

### All locks found

Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms)
------ | ------ | ---- | ------- | --- | ---- | ------------------
`public` | `books` | `AccessExclusiveLock` | Table | 1 | ❌ | 20
`public` | `books` | `ShareUpdateExclusiveLock` | Table | 1 | ✅ | 10

### Dangerous locks found

- `AccessExclusiveLock` would block the following operations on `public.books`:
  + `SELECT`
  + `FOR UPDATE`
  + `FOR NO KEY UPDATE`
  + `FOR SHARE`
  + `FOR KEY SHARE`
  + `UPDATE`
  + `DELETE`
  + `INSERT`
  + `MERGE`

## Statement number 1 for 10 ms

### SQL

```sql
alter table books add constraint check_title_not_null check (title is not null) not valid;
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

Schema | Object | Mode | Relkind | OID | Safe
------ | ------ | ---- | ------- | --- | ----
`public` | `books` | `AccessExclusiveLock` | Table | 1 | ❌

### Hints

#### Taking dangerous lock without timeout

ID: `dangerous_lock_without_timeout`

A lock that would block many common operations was taken without a timeout. This can block all other operations on the table indefinitely if any other transaction holds a conflicting lock while `idle in transaction` or `active`. A safer way is: Run `SET lock_timeout = '2s';` before the statement and retry the migration if necessary.

The statement took `AccessExclusiveLock` on the Table `public.books` without a timeout. It blocks `SELECT`, `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`, `UPDATE`, `DELETE`, `INSERT`, `MERGE` while waiting to acquire the lock.

## Statement number 2 for 10 ms

### SQL

```sql
alter table books validate constraint check_title_not_null;
```

### Locks at start

Schema | Object | Mode | Relkind | OID | Safe
------ | ------ | ---- | ------- | --- | ----
`public` | `books` | `AccessExclusiveLock` | Table | 1 | ❌

### New locks taken

Schema | Object | Mode | Relkind | OID | Safe
------ | ------ | ---- | ------- | --- | ----
`public` | `books` | `ShareUpdateExclusiveLock` | Table | 1 | ✅

### Hints

#### Running more statements after taking `AccessExclusiveLock`

ID: `holding_access_exclusive`

A transaction that holds an `AccessExclusiveLock` started a new statement. This blocks all access to the table for the duration of this statement. A safer way is: Run this statement in a new transaction.

The statement is running while holding an `AccessExclusiveLock` on the Table `public.books`, blocking all other transactions from accessing it.
