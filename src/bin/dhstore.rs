use std::fs::File;
use std::io::{self, Write};
use std::process;

use clap::{App, Arg, SubCommand, crate_version};
use log::{Level, error};

use dhstore;
use dhstore::errors::Error;
use dhstore::hash::ID;
use dhstore::logger::init;

fn main() {
    let verbose = &Arg::with_name("verbose")
        .short("v")
        .multiple(true)
        .help("Augment verbosity level");
    let store_args = &[
        Arg::with_name("store")
            .short("d")
            .takes_value(true)
            .value_name("PATH")
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
        .subcommand(SubCommand::with_name("verify")
                    .about("Verifies the store (checks for invalid values)")
                    .arg(verbose)
                    .args(store_args))
        .subcommand(SubCommand::with_name("gc")
                    .about("Verifies the store and deletes garbage \
                            (unreachable objects and blobs)")
                    .arg(verbose)
                    .args(store_args))
        .subcommand(SubCommand::with_name("add")
                    .about("Add a file or directory")
                    .arg(verbose)
                    .args(store_args)
                    .arg(Arg::with_name("INPUT")
                         .required(true)
                         .help("Input file")))
        .subcommand(SubCommand::with_name("show")
                    .about("Pretty-print an object")
                    .arg(verbose)
                    .args(store_args)
                    .arg(Arg::with_name("ID")
                         .required(true)
                         .help("ID of object to print from"))
                    .arg(Arg::with_name("--depth")
                         .takes_value(true)
                         .value_name("DEPTH")
                         .help("Maximum recursion depth")))
        .subcommand(SubCommand::with_name("blob_add")
                    .about("Low-level; add a blob from a file or stdin")
                    .arg(verbose)
                    .args(store_args)
                    .arg(Arg::with_name("INPUT")
                         .required(true)
                         .help("Input file or \"-\" for stdin")))
        .subcommand(SubCommand::with_name("blob_get")
                    .about("Low-level; get a blob from the store by its ID")
                    .arg(verbose)
                    .args(store_args)
                    .arg(Arg::with_name("ID")
                         .required(true)
                         .help("ID of the blob to print")))
        .get_matches();

    let mut level = matches.occurrences_of("verbose");
    if let (_, Some(m)) = matches.subcommand() {
        level += m.occurrences_of("verbose");
    }
    let level = match level {
        0 => Level::Warn,
        1 => Level::Info,
        2 => Level::Debug,
        3 | _ => Level::Trace,
    };
    init(level).unwrap();

    match matches.subcommand() {
        (_, None) => {
            error!("No command specified.");
            process::exit(1);
        }
        (command, Some(matches)) => {
            if let Err(e) = run_command(command, matches) {
                error!("{}", e);
                process::exit(1);
            }
        }
    }
}

fn run_command(command: &str, matches: &clap::ArgMatches)
        -> dhstore::errors::Result<()> {
    let get_store = ||
            -> dhstore::errors::Result<dhstore::Store<dhstore::FileBlobStorage,
                                       dhstore::MemoryIndex>> {
        dhstore::open(matches.value_of_os("store")
                      .unwrap_or_else(|| ".".as_ref()))
    };
    match command {
        "init" => {
            let path = matches.value_of_os("store")
                .unwrap_or_else(|| ".".as_ref());
            dhstore::create(path)
        }
        "verify" => {
            get_store()?.verify()
        }
        "gc" => {
            get_store()?.collect_garbage()
        }
        "add" => {
            let id = get_store()?.add(matches.value_of_os("INPUT").unwrap())?;
            println!("{}", id);
            Ok(())
        }
        "show" => {
            let store = get_store()?;
            let id = ID::from_str(matches.value_of("ID").unwrap().as_bytes())
                .ok_or(Error::InvalidInput("Input is not a valid ID"))?;
            let depth = if let Some(arg) = matches.value_of_lossy("DEPTH") {
                match arg.parse() {
                    Ok(i) => Some(i),
                    Err(_) => {
                        return Err(
                            Error::InvalidInput("Invalid number for --depth"));
                    }
                }
            } else {
                None
            };
            store.print_object(&id, depth)
        }
        "blob_add" => {
            let mut store = get_store()?;
            let file = matches.value_of_os("INPUT").unwrap();
            let id = if file == "-" {
                store.add_blob(io::stdin())
            } else {
                let fp = File::open(file)
                    .map_err(|e| ("Cannot open file for reading", e))?;
                store.add_blob(fp)
            }?;
            println!("{}", id);
            Ok(())
        }
        "blob_get" => {
            let store = get_store()?;
            let id = ID::from_str(matches.value_of("ID").unwrap().as_bytes())
                .ok_or(Error::InvalidInput("Input is not a valid ID"))?;
            match store.get_blob(&id)? {
                Some(blob) => {
                    io::stdout().write_all(&blob)
                        .map_err(|e| ("Error writing to stdout", e))?;
                }
                None => {
                    write!(io::stderr(), "Blob not found").unwrap();
                    process::exit(1);
                }
            }
            Ok(())
        }
        _ => panic!("Missing code for command {}", command),
    }
}
