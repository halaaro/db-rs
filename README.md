# DB

<img src="doc/db.png">

A tool make interacting with a database much easier to script and automate.

> This project is a major work in progress. This README is to guide development and nothing is expected to work (yet).

> The initial design and implementation supports only Microsoft SQL-Server. Other databases will be added as time permits and the idea is to not make anything specific to any particular database, unless needed.

### Connecting

Connect to a database using `-x` flag followed by a connection string.
To make the connection persistent, use `-d` to keep the connection open in the background and used by subsequent commands. Without `-d` the connection will immediately close after running any other operations (such as queries or commands, see below for details).

Example connecting to a SQL Server database:

```sh
$ db -d -x "Server=localhost;User=SA;Password=P@ssw0rd;"
default connection successful
```

Optionally the connection can be named with `-n` flag:

```sh
$ db -d -n dev1 -x "Server=localhost;User=SA;Password=P@ssw0rd;"
dev1 connection successful
```

### Querying

The active database connection can be queried by using the `-q` flag followed by the query.

```sh
$ db -q "SELECT id, name FROM users"
 id |    name
----|--------------
  1 | John Johnson
  2 | Paul Paulson
```

Standard in can be used instead by passing `-q -`:

```sh
$ echo "SELECT 1" | db -q - -o users.json
1
```

A new connection can be created by using `-x` (see above). To use a connection other than `default`, use the `-n` argument to specify the existing connection:

```sh
$ db -n conn1 -q "SELECT 1"
1
```

Output can be saved to a file with the `-o` flag. Output format is inferred from the output file name, defaulting to CSV if an unknown extension. To change the output format `-f <format>` can be specified. Currently supported output formats:
* `csv` (default)
* `json`
* `markdown`

### Commands

Like queries, commands can be used with the `-c` argument:

```sh
$ db -c "INSERT INTO users (id, name, email) VALUES (1, 'jake', 'jake@kagaru.com')"
1 row affected
```

Command input can be read from a file, just like queries by using `-c -`:

```sh
$ db -c -
10 rows affected
```

### Parameterized Commands

Parameterized commands can also be performed using the syntax `$<variable>` and passing arguments by name with `-p <variable>=<value>` syntax.

```sh
$ db -c "INSERT INTO users (id, name, email) VALUES ($id, $name, $email)" \
  -p id=1 -p "name=Phillip Porter" -p "email=phil@porter.net"
1 row affected
```

Instead of passing them one-by-one, parameters can be read from a file using `-p <filename>` where supported file formats are `toml`, `json`, `csv`, and `markdown` tables.

If more than one row of data is to be used, the `-s` option enables streaming mode where each row of the paramter file is used and the command runs multiple times. If batch size is important, this can be specified with the `-b <max-batch-size>` paramter.

```sh
$ db -c "INSERT INTO users (id, name, email) VALUES ($id, $name, $email)" \
  -s -p ./users.csv
```

### Generating SQL

If generating a SQL script is needed, the `-o` flag can be used with a filename ending in `.sql` or by specifying `-f sql`:

```sh
$ db -c "INSERT INTO users (id, name, email) VALUES ($id, $name, $email)" \
  -s -p ./users.csv -o insert-users.sql
```

Example writing generated to standard out (and not executing):

```sh
$ db -c "INSERT INTO users (id, name, email) VALUES ($id, $name, $email)" \
  -s -p ./users.csv -o -
INSERT INTO users (id, name, email) VALUES (1, 'Phillip Porter', 'phil@porter.net')
```

## Building

Nothing special should be required:

```sh
cargo build -r
```

## Attribution

Inspired by the likes of [jq](https://github.com/jqlang/jq).

## License

The contents of this repository are dual-licensed under the _MIT OR Apache 2.0_
License. That means you can chose either the MIT license or the Apache-2.0
license when you re-use this code. See `LICENSE-MIT` or `LICENSE-APACHE` for
more information on each specific license.

