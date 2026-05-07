# pproxy

`pproxy` is a small TCP proxy server written in Rust.

It reads a YAML config file, listens on one or more local ports, and forwards traffic to a target host and port.

## Features

- Multiple proxy rules in one config file
- Configurable listen host and port
- Configurable target host and port
- Rotating log files
- `-v`, `--v`, `-ver`, `--ver`, and `-version` compatibility
- `-V` / `--version` output with build hash and build date when available

## Build

### Windows

```bat
build.bat
```

### Linux / macOS

```bash
./build.sh
```

### Cargo

```bash
cargo build --release
```

## Run

By default, the application reads `pproxy.yaml` in the current directory.

```bash
cargo run -- --config pproxy.yaml
```

Or directly with the built binary:

```bash
target/release/pproxy --config pproxy.yaml
```

Version output:

```bash
pproxy --version
```

## Configuration

Example:

```yaml
proxy_list:
  - listen_host: 0.0.0.0
    listen_port: 1890
    target_host: 127.0.0.1
    target_port: 1853

log_dir: logs
log_file: pproxy.log
log_max_size: 3
log_max_backups: 100
log_max_age: 28
```

### Proxy rule

- `listen_host`: local bind host, default `0.0.0.0`
- `listen_port`: local bind port
- `target_host`: upstream host
- `target_port`: upstream port

### Log settings

- `log_dir`: log output directory, default `./logs`
- `log_file`: log file name, default `log.log`
- `log_max_size`: max log size in MiB, default `2`
- `log_max_backups`: number of rotated log files to keep, default `100`
- `log_max_age`: delete rotated logs older than this many days, default `28`

## Notes

- `build.rs` embeds build metadata into the binary at compile time.
- If a build hash is available, version output shows both the hash and the build date.
- If no build hash is available, version output shows only the build date.
