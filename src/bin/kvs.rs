use std::process;

use clap::{arg, Command};

fn main() {
    let matches = Command::new("kvs")
        .version(env!("CARGO_PKG_VERSION")) //Print the version
        .author("xiaSang <3188674636@qq.com>")
        .about("A key-value store")
        .subcommand(
            Command::new("set")
                .about("Set the value of a string key to a string")
                .arg(arg!(<KEY>))
                .arg(arg!(<VALUE>)),
        )
        .subcommand(
            Command::new("get")
                .about("Get the string value of a given string key")
                .arg(arg!(<KEY>)),
        )
        .subcommand(
            Command::new("rm")
                .about("Remove a given key")
                .arg(arg!(<KEY>)),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("set", sub_matchs)) => {
            println!(
                "{}{}",
                sub_matchs.get_one::<String>("KEY").unwrap(),
                sub_matchs.get_one::<String>("VALUE").unwrap()
            );
            eprintln!("unimplemented!");
            process::exit(-1)
        }
        Some(("get", sub_matchs)) => {
            println!("{}", sub_matchs.get_one::<String>("KEY").unwrap(),);
            eprintln!("unimplemented!");
            process::exit(-1)
        }
        Some(("rm", sub_matchs)) => {
            println!("{}", sub_matchs.get_one::<String>("KEY").unwrap(),);
            eprintln!("unimplemented!");
            process::exit(-1)
        }
        _ => process::exit(-1),
    }
}
