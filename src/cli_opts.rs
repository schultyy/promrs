use clap::{Arg, App};

pub fn print_local() -> bool {
    let matches = App::new("promrs")
    .version("0.1")
    .author("Jan Schulte. <janschulte@fastmail.com>")
    .arg(Arg::with_name("local")
         .short("l")
         .long("local")
         .help("Prints traces to local stdout instead of jaeger"))
    .get_matches();

    matches.occurrences_of("local") > 0
}