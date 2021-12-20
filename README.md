# promrs

A very incomplete Prometheus implementation in Rust.

This project's sole purpose is to learn working with Tokio's tasks and building a rudimentary data store.

## Usage

Run the application with:

```
$ cargo run
```

You need to have **Node Exporter** running on `localhost:9100/metrics`. promrs only crawls this single, hard-coded Url at the moment.

It's possible to query data via `http://localhost:3030/query?key=metric`.

## License

MIT