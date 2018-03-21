use colored::Colorize;

use config::{self, Config};

pub fn pretty(config: &Config) {
    // SERVER

    println!();
    println!(
        "{} {}",
        "[hunk]".bright_white().bold(),
        "listening".bright_green().bold()
    );
    println!("folder:  {}", config.server.root.to_str().unwrap().bright_white().bold());
    println!(
        "address: {}{}",
        "http://".bright_white(),
        config.server.addr.to_string().bright_white().bold()
    );

    // GZIP

    println!(
        "- gzip: {}",
        match config.gzip.as_ref() {
            None => "off".red().bold().to_string(),
            Some(_) => format!("{}", "on".green().bold()),
        }
    );

    // CORS

    println!(
        "- cors: {}",
        match config.cors.as_ref() {
            None => "off".red().bold().to_string(),
            Some(opts) => {
                let mut s = format!("{}", "on".green().bold());
                s.push(' ');
                let origin = match opts.origin {
                    config::Origin::Any =>
                        "*".to_string(),
                    config::Origin::Few(ref urls) =>
                        format!("{:?}", urls.iter().map(|u| format!("{}", u)).collect::<Vec<String>>()),
                };
                s.push_str(format!("origin={}", origin.bold()).as_ref());
                s
            }
        }
    );

    //  LOG

    println!(
        "- log: {}",
        match config.log {
            None => "off".red().bold().to_string(),
            Some(_) => {
                let mut s = format!("{}", "on".green().bold());
                s.push(' ');
                s.push_str(&format!("dst={}", "stdout".bold()));
                s
            }
        }
    );

    // BROWSE

    println!(
        "- browse: {}",
        match config.browse {
            None => "off".red().bold().to_string(),
            Some(_) => format!("{}", "on".green().bold()),
        }
    );
}