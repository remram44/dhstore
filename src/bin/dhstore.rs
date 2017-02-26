#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate termcolor;

extern crate dhstore;

use dhstore::log::init;

use clap::{App, Arg, SubCommand};
use log::LogLevel;

fn main() {
    let verbose = &Arg::with_name("verbose")
        .short("v")
        .multiple(true)
        .help("Augment verbosity level");
    let store_args = &[
        Arg::with_name("store")
            .short("d")
            .value_name("PATH")
            .takes_value(true)
            .help("Location of the store"),
    ];
    let matches = App::new("dhstore")
        .about("dhstore command-line client")
        .version(crate_version!())
        .author("Remi Rampin <remirampin@gmail.com>")
        .arg(verbose)
        .subcommand(SubCommand::with_name("init")
                    .about("Creates a new store")
                    .arg(verbose)
                    .args(store_args))
        .get_matches();

    let mut level = matches.occurrences_of("verbose");
    if let (_, Some(m)) = matches.subcommand() {
        level += m.occurrences_of("verbose");
    }
    let level = match level {
        0 => LogLevel::Warn,
        1 => LogLevel::Info,
        2 => LogLevel::Debug,
        3 | _ => LogLevel::Trace,
    };
    init(level).unwrap();

    let store = dhstore::open("store");
}
