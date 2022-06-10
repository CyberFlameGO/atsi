#[macro_use]
extern crate log;

mod engine;
mod util;

use crate::util::SyncResult;

use clap::{Arg, Command};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> SyncResult<()> {
    let start = Instant::now();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    engine::alpine::download_rootfs(engine::alpine::VERSION).await?;
    debug!(
        "cached alpine rootfs at: {}",
        engine::alpine::rootfs_path(engine::alpine::VERSION).display()
    );
    engine::slirp::download_slirp4netns().await?;
    debug!(
        "cached slirp4netns at: {}",
        engine::slirp::bin_path().display()
    );

    let matches = Command::new("@")
        .subcommand(
            Command::new("run")
                .visible_alias("r")
                .arg(
                    Arg::new("command")
                        .takes_value(true)
                        .help("The command to run inside of the container. Defaults to `sh`.")
                        .default_value("sh")
                        .long_help("The command to run inside of the container. This command will be pid 1\
                            inside the container, and will have a bare-minimum environment set up.\n\
                            \n\
                            The default value for the command is `sh`, to just always get a shell.\n\
                            \n\
                            Examples:\n\
                            - Get a shell: `@ run`\n\
                            - Install Python 3: `@ run -P python3`")
                )
                // .arg(Arg::new("detach").short('d').required(false))
                .arg(
                    Arg::new("immutable")
                        .short('i')
                        .long("immutable")
                        .required(false)
                        .takes_value(false)
                        .help("Makes the container's rootfs immutable (read-only).")
                        ,
                )
                .arg(
                    Arg::new("package")
                        .short('P')
                        .multiple_occurrences(true)
                        .takes_value(true)
                        .help("Specify a package to install. Can be specified multiple times.")
                        ,
                )
                .arg(
                    Arg::new("port")
                        .short('p')
                        .multiple_occurrences(true)
                        .takes_value(true)
                        .help("Expose a port to the host. Format is outer:inner, ex. `8080:8081`.")
                        ,
                )
                .arg(
                    Arg::new("rw")
                        .long("rw")
                        .multiple_occurrences(true)
                        .takes_value(true)
                        .help("Mount a file/directory read-write. Format is source:target, ex. `/home/me/file:/file`.")
                        ,
                )
                .arg(
                    Arg::new("ro")
                        .long("ro")
                        .multiple_occurrences(true)
                        .takes_value(true)
                        .help("Mount a file/directory read-only. Format is source:target, ex. `/home/me/file:/file`.")
                        ,
                ),
        )
        .subcommand(Command::new("ps"))
        .get_matches();

    match matches.subcommand_name() {
        Some("run") => {
            let matches = matches.subcommand_matches("run").unwrap();
            let command = matches.value_of("command").unwrap();
            let detach: bool = false; // matches.is_present("detach");
            let packages: Vec<String> = matches
                .values_of("package")
                .map_or(vec![], |v| v.map(|f| f.to_string()).collect());
            let ports: Vec<(u16, u16)> = matches.values_of("port").map_or(vec![], |v| {
                v.map(|p| {
                    let slice: Vec<&str> = p.split(':').collect();
                    (
                        slice[0].parse().expect("outer port must be valid u16"),
                        slice[1].parse().expect("inner port must be valid u16"),
                    )
                })
                .collect()
            });
            let immutable: bool = matches.is_present("immutable");
            let rw_mounts: Vec<(String, String)> = matches.values_of("rw").map_or(vec![], |v| {
                v.map(|p| {
                    let slice: Vec<String> = p.split(':').map(|s| s.to_string()).collect();
                    (slice[0].clone(), slice[1].clone())
                })
                .collect()
            });
            let ro_mounts: Vec<(String, String)> = matches.values_of("ro").map_or(vec![], |v| {
                v.map(|p| {
                    let slice: Vec<String> = p.split(':').map(|s| s.to_string()).collect();
                    (slice[0].clone(), slice[1].clone())
                })
                .collect()
            });

            engine::Engine::new(start)
                .run(engine::RunOpts {
                    command: command.to_string(),
                    packages,
                    detach,
                    ports,
                    immutable,
                    rw_mounts,
                    ro_mounts,
                })
                .await?;
        }
        Some("ps") => {
            engine::Engine::new(start).ps().await?;
        }
        _ => {}
    }

    Ok(())
}
