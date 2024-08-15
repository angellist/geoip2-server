# geoip2-server

This is a server implementing the MaxMind Geoip2 lookup API.

## Usage

```Shell
git clone https://github.com/angellist/geoip2-server.git
cd geoip2-server
cargo run --release -- --bind 0.0.0.0 --port 3000 --database /path/to/geolite2.mmdb
```

## License

This project is licensed under the [MIT license](license).
