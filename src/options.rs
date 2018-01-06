use config::Config;

// Not sure if this struct makes much sense, yet.
//
// The idea: Options are the result of validating a Config object and
// it represents only the things the handler cares about.
//
// Validation should live here and values should be
// lifted into their final structs (like u32 -> Compression(u32))
// so that the handler doesn't have to do it.

#[derive(Clone)]
pub struct Options {
    pub gzip: Option<Gzip>,
    pub cache: Option<Cache>,
}

#[derive(Clone)]
pub struct Gzip {
    pub level: ::flate2::Compression,
    pub threshold: u64,
}

#[derive(Clone)]
pub struct Cache {
    pub max_age: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            gzip: None,
            cache: None,
        }
    }
}

impl Options {
    pub fn new(config: Config) -> Result<Options, String> {
        let mut o = Options::default();

        if let Some(opts) = config.gzip {
            if opts.level < 1 || opts.level > 9 {
                return Err(format!("gzip.level must be 1-9. actual={}", opts.level));
            }

            o.gzip = Some(Gzip {
                level: ::flate2::Compression::new(opts.level),
                threshold: opts.threshold,
            })
        };

        if let Some(opts) = config.cache {
            o.cache = Some(Cache {
                max_age: opts.max_age,
            })
        }

        Ok(o)
    }
}
