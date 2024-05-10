CREATE DATABASE test_db;
CREATE DATABASE "example-db";
CREATE DATABASE "snapshot-test";

\c test_db

CREATE TABLE books
(
    id    SERIAL PRIMARY KEY,
    title text,
    price integer
);

CREATE TABLE for_checking_modified_constraints (
    id    SERIAL PRIMARY KEY,
    title text check (length(title) < 10),
    book_id integer references books(id)
);

\c "example-db"

CREATE TABLE books
(
    id    SERIAL PRIMARY KEY,
    title text
);

\c "snapshot-test"

CREATE TABLE books
(
    id    SERIAL PRIMARY KEY,
    title text,
    price integer
);

