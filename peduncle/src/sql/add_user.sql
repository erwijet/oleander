INSERT INTO oleander.users(username, first_name, last_name, pwd)
VALUES ($1, $2, $3, $4)

RETURNING $table_fields;