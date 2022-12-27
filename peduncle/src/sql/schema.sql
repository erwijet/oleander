DROP SCHEMA IF EXISTS oleander CASCADE;
CREATE SCHEMA oleander;

CREATE TABLE oleander.users (
    id          BIGSERIAL PRIMARY KEY,
    first_name  VARCHAR(200) NOT NULL,
    last_name   VARCHAR(200) NOT NULL,
    username    VARCHAR(200) NOT NULL,
    pwd         VARCHAR(200) NOT NULL,

    UNIQUE (username)
)