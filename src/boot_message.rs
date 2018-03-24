
/// The boot message is the pretty heads-up that prints on server boot if stdout is tty.

use colored::Colorize;

use config::{self, Config, Site, CorsOrigin};
use host::Host;

fn pretty_site(site: &Site) {
    println!(
        "site: [{hosts}]",
        hosts = site.host.iter()
            .map(Host::to_string)
            .map(|s| s.bright_white().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );

    // PROXY

    println!(
        "- proxy:  {}",
        match site.url {
            None => "off".to_string(),
            Some(ref url) => format!("{}   -> {}", "on".green().bold(), url),
        }
    );

    // GZIP

    println!(
        "- gzip:   {}",
        match site.gzip.as_ref() {
            None => "off".to_string(),
            Some(_) => format!("{}", "on".green().bold()),
        }
    );

    // CORS

    println!(
        "- cors:   {}",
        match site.cors.as_ref() {
            None => "off".to_string(),
            Some(opts) => {
                let mut s = format!("{}", "on".green().bold());
                s.push(' ');
                let origin = match opts.origin {
                    CorsOrigin::Any =>
                        "*".to_string(),
                    CorsOrigin::Few(ref urls) =>
                        format!("{:?}", urls.iter().map(|u| format!("{}", u)).collect::<Vec<String>>()),
                };
                s.push_str(format!("origin={}", origin.bold()).as_ref());
                s
            }
        }
    );

    //  LOG

    println!(
        "- log:    {}",
        match site.log {
            None => "off".to_string(),
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
        match site.browse {
            None => "off".to_string(),
            Some(_) => format!("{}", "on".green().bold()),
        }
    );

    // ROOT

    println!(
        "- root:   {}",
        match site.root {
            None => "off".to_string(),
            Some(ref root) =>
                format!(
                    "{} {}",
                    "on".green().bold(),
                    root.to_str().unwrap_or("").to_string().bright_white()
                )
        }
    );
}

pub fn pretty(config: &Config) {
    // SERVER

    println!();
    println!(
        "{} {} on {}",
        "[prox]".bright_white().bold(),
        "listening".bright_green().bold(),
        config.server.bind.to_string().replace("127.0.0.1", "localhost").bright_white().bold()
    );

    // SITES

    for site in &config.sites {
        pretty_site(site)
    }
}