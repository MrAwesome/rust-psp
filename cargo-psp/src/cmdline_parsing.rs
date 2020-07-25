use clap::{App, AppSettings, Arg, SubCommand};

const XARGO_TOKEN: &str = "xargo";
const BUILD_STD_TOKEN: &str = "build-std";

const RELEASE_PROFILE_TOKEN: &str = "release_profile";
const NO_STD_TOKEN: &str = "no_std";
const BUILD_METHOD_TOKEN: &str = "build_method";
const LOCAL_LIBC_TOKEN: &str = "local_libc";
const LOCAL_RUST_SRC_TOKEN: &str = "local_rust_src";
const CARGO_ARGS_TOKEN: &str = "cargo_args";

#[derive(Debug, Default)]
pub(crate) struct CargoPspOptions {
    pub(crate) std: bool,
    pub(crate) build_method: BuildMethod,
    pub(crate) local_libc: Option<String>,
    pub(crate) local_rust_src: Option<String>,
    pub(crate) release: bool,
    pub(crate) cargo_args: Vec<String>,
}

#[derive(Debug)]
pub(crate) enum BuildMethod {
    Xargo,
    CargoBuildStd,
}

impl Default for BuildMethod {
    fn default() -> Self {
        Self::Xargo
    }
}

impl From<Option<&str>> for BuildMethod {
    fn from(arg: Option<&str>) -> Self {
        match arg {
            Some(s) if s == XARGO_TOKEN => Self::Xargo,
            Some(s) if s == BUILD_STD_TOKEN => Self::CargoBuildStd,
            _ => Self::default(),
        }
    }
}

pub(crate) fn parse_cmdline() -> CargoPspOptions {
    let main_matcher = App::new("cargo-psp")
        .setting(AppSettings::TrailingVarArg)
        .bin_name("cargo")
        .long_about(
            "NOTE: This command expects to be run as `cargo psp`, with the\
            cargo-psp executable living somewhere in your $PATH. \
            Running cargo-psp by itself will almost certainly not \
            work the way you expect."
        )
        .subcommand(SubCommand::with_name("psp")
            .about("Compiles Rust code into an EBOOT.PBP file ready to run on a PSP.")
            .arg(
                Arg::with_name(NO_STD_TOKEN)
                    .short("n")
                    .long("no-std")
                    .help("Do not build the standard library during the bootstrapping process."),
            )
            .arg(
                Arg::with_name(RELEASE_PROFILE_TOKEN)
                    .long("release")
                    .help("Whether or not to build in release mode."),
            )
            .arg(
                Arg::with_name(BUILD_METHOD_TOKEN)
                    .short("b")
                    .long("build-method")
                    .takes_value(true)
                    .possible_values(&[XARGO_TOKEN, BUILD_STD_TOKEN])
                    .default_value(XARGO_TOKEN)
                    .help("Which cargo wrapper to use for bootstraping core/std."),
            )
            .arg(
                Arg::with_name(LOCAL_LIBC_TOKEN)
                    .long("libc-crate")
                    .takes_value(true)
                    .help("Location on disk of the libc crate. Note that this does *not* point at the src/ directory, but the crate root."),
            )
            .arg(
                Arg::with_name(LOCAL_RUST_SRC_TOKEN)
                    .long("rust-src")
                    .takes_value(true)
                    .help("Location on disk of a local checkout of the rust src. Note that this points at the src/ directory."),
            )
            .arg(
                Arg::with_name(CARGO_ARGS_TOKEN)
                    .multiple(true)
                    .takes_value(true)
                    .help("Arguments to pass along to Xargo."),
            )
        )
        .get_matches();

    match main_matcher.subcommand() {
         (x, Some(m)) if x == "psp" => {
            let std = !m.is_present(NO_STD_TOKEN);
            let build_method = BuildMethod::from(m.value_of(BUILD_METHOD_TOKEN));
            let local_libc = m.value_of(LOCAL_LIBC_TOKEN).map(String::from);
            let local_rust_src = m.value_of(LOCAL_RUST_SRC_TOKEN).map(String::from);
            let release = m.is_present(RELEASE_PROFILE_TOKEN);

            let cargo_args: Vec<String> = m
                .values_of(CARGO_ARGS_TOKEN)
                .map(|vals| vals.map(Into::into).collect())
                .unwrap_or_else(|| vec![]);

            CargoPspOptions {
                std,
                build_method,
                local_libc,
                local_rust_src,
                release,
                cargo_args,
            }
        },
        _ => {
            eprintln!("[ERROR]: cargo-psp expects to be run as `cargo psp`, with cargo-psp somewhere in your $PATH.");
            std::process::exit(1);
         }
        
    }
}
