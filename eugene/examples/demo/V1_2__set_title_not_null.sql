SET LOCAL LOCK_TIMEOUT = '1s';

ALTER TABLE BOOKS
    ALTER COLUMN TITLE SET NOT NULL;

ALTER TABLE BOOKS
    ADD CONSTRAINT TITLE_LENGTH
        CHECK (LENGTH(TITLE) <= 100);