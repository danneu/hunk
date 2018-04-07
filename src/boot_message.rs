/// The boot message is the pretty heads-up that prints on server boot if stdout is tty.
use colored::Colorize;

use config::{Config, CorsOrigin, Site, Serve};
use host::Host;

fn pretty_site(site: &Site) {
    println!(
        "site: [{hosts}]",
        hosts = site.host
            .iter()
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
                    CorsOrigin::Any => "*".to_string(),
                    CorsOrigin::Few(ref urls) => format!(
                        "{:?}",
                        urls.iter()
                            .map(|u| format!("{}", u))
                            .collect::<Vec<String>>()
                    ),
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
                format!(
                    "{} to={}",
                    "on".green().bold(),
                    "stdout".bold(),
                )
            }
        }
    );

    // SERVE

    println!(
        "- serve:  {}",
        match site.serve {
            None => "off".to_string(),
            Some(Serve { ref root, dotfiles, browse }) => format!(
                "{} root=\"{}\"{}{}",
                "on".green().bold(),
                root.to_str().unwrap_or("").to_string().bright_white().bold(),
                if browse { " +browse" } else { "" }.bold(),
                if dotfiles { " +dotfiles" } else { "" }.bold(),
            ),
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
        config
            .server
            .bind
            .to_string()
            .replace("127.0.0.1", "localhost")
            .bright_white()
            .bold()
    );

    // SITES

    for site in &config.sites {
        pretty_site(site)
    }

    if config.sites.is_empty() {
        println!("- No sites configured")
    }
}
