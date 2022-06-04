psql -v ON_ERROR_STOP=1 --username "postgres" --dbname "postgres" <<-EOSQL
  BEGIN;
	CREATE TABLE user_table(
		id SERIAL NOT NULL PRIMARY KEY,
		create_time DATE,
		update_time DATE,
		name_user VARCHAR(128) NOT NULL,
		privilege INTEGER DEFAULT 3 NOT NULL,
		hashed_password bytea NOT NULL,
		description VARCHAR(512) DEFAULT ''
	);
	CREATE TABLE token_table(
		id SERIAL NOT NULL PRIMARY KEY,
		CONSTRAINT FK_user FOREIGN KEY (id) REFERENCES user_table(id),
		chipper CHAR(32) NOT NULL
	);
	CREATE TABLE question_table(
		id SERIAL NOT NULL PRIMARY KEY,
		title VARCHAR(128) DEFAULT '',
		description VARCHAR(1024) DEFAULT ''
	);
	CREATE TABLE question_user(
		id SERIAL NOT NULL PRIMARY KEY,
		CONSTRAINT FK_user FOREIGN KEY (id) REFERENCES user_table(id),
		CONSTRAINT FK_question FOREIGN KEY (id) REFERENCES question_table(id)
	);
  COMMIT;
EOSQL