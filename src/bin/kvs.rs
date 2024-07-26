use clap::{arg, Command};
use kvs::{KvStore, KvStoreError, Result};
use std::{env, process};
fn main() -> Result<()> {
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
    let mut store = KvStore::open(env::current_dir()?)?;
    match matches.subcommand() {
        Some(("set", sub_matchs)) => {
            let key = sub_matchs.get_one::<String>("KEY").unwrap();
            let value = sub_matchs.get_one::<String>("VALUE").unwrap();
            if let Err(err) = store.set(key.to_owned(), value.to_owned()) {
                println!("{:?}", err);
                process::exit(-1);
            };
        }
        Some(("get", sub_matchs)) => {
            let key = sub_matchs.get_one::<String>("KEY").unwrap();
            if let Err(err) = store.get(key.to_owned()) {
                println!("{:?}", err);
                process::exit(-1);
            };
            match store.get(key.to_owned())? {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        Some(("rm", sub_matchs)) => {
            let key = sub_matchs.get_one::<String>("KEY").unwrap();
            if let Err(KvStoreError::KeyNotFound) = store.remove(key.to_owned()) {
                println!("Key not found");
                process::exit(-1);
            };
        }
        _ => process::exit(-1),
    }
    Ok(())
}
