# simple-ftp-server
Simple FTP server I have written in Rust in 2021

# Features
- Basic FTP operations such as sending, retrieving and listing listing files
- Supports FTP authentication
- Configurable through toml file and commandline arguments

# Anti-features
- It is synchronous code running on one thread

# Configuration
For now, you can configure server, logging and users.
## TOML File
```toml
[server]
port = 21
ip = "127.0.0.1"

[log.file]
path = "test.log"
level = "debug"

[log.console]
level = "debug"

[user.anonymous]
password = "anonymous@example.com"
directory = "anon"

[user.alice]
password = "donttellbob"
directory = "alice"
```
## Console
You can check available options by running program with `--help` flag
```
ftp-server 0.1.0
Stanisław Borowy <stabor@startmail.com>

USAGE:
    ftp-server [OPTIONS]

OPTIONS:
    -c, --config <config>    Sets the path to toml configuration file
    -h, --help               Print help information
    -i, --ip <IP>            Sets the ip address server will try to use
    -p, --port <PORT>        Sets the port number the server will try to bind to
    -V, --version            Print version information
```
