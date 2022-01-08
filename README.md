# Orienteering Vlaanderen Rankings

This repository contains the CGI program and web page to generate the Forest Cup, City Cup and Kampioen rankings.

## Populating the database

The results of an event are stored in an sqlite database.
The ranking is generated on-the-fly.

Inserting data into the database is done using the `load` binary.

```bash
$ cargo run --bin load -- --season 2022 data/20211121.json
```

The JSON data should be downloaded from the Helga Webres https://helga-o.com/webres/ws.php?lauf=? API.

## Preparing the frontend

```bash
$ npm ci
$ npm run build
```

## Serve the ranking

```bash
$ mkdir cgi-bin
$ cargo build --release --bin cup-cgi
$ cp target/release/cup-cgi cgi-bin/
$ SCRIPT_FILENAME=cgi-bin/cup-cgi python -m http.server --cgi
```

## Release

```bash
$ rsync -rv *.html favicon.ico ov.sqlite dist images cgi-bin mole.hoekx.be:/srv/http/rankings.orienteering.vlaanderen/
```

## Development

Ensure Clippy and eslint are happy:

```bash
$ npm run lint
$ cargo clippy
```
