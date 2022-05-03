use clap::{Command, command, Arg}

pub fn build_cli() -> Command {
    command!()
        .arg(Arg::new()
             .long("ip")
             .help("IP address in form x.x.x.x of network interface the server "
                   "will try to listen on. Defaults to LOCALHOST."))
        .arg(Arg::new()
             .long("port")
             .help("Sets the port on which server will listen for incoming "
                   "control connections. Should be a valid port number. "
                   "Will listen on random available port if set to 0."))

}
