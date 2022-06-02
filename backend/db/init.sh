psql -v ON_ERROR_STOP=1 --username "postgres" --dbname "postgres" <<-EOSQL
  BEGIN;
	CREATE TABLE user_table(
		id SERIAL NOT NULL PRIMARY KEY,
		create_time DATE,
		update_time DATE,
		name_user VARCHAR(128) NOT NULL,
		privilege INTEGER DEFAULT 3 NOT NULL,
		hashed_password bytea NOT NULL,
		description VARCHAR(512) DEFAULT ""
	);
  COMMIT;
EOSQL