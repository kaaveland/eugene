drop index concurrently if exists books_concurrently_test_idx;
create index concurrently books_concurrently_test_idx on books(title);
