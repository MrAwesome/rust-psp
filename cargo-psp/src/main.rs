use cargo_metadata::MetadataCommand;
use std::{
    env, fs,
    io::{self, ErrorKind, Read, Write},
    process::{self, Command, Stdio},
    thread,
};

mod psp_xargo;
use psp_xargo::get_xargo_toml_text;

mod psp_config;
use psp_config::PspConfig;

mod cmdline_parsing;
use cmdline_parsing::parse_cmdline;

const SUBPROCESS_ENV_VAR: &str = "__CARGO_PSP_RUN_XARGO";
const TARGET_TRIPLE: &str = "mipsel-sony-psp";

fn main() {
    if env::var(SUBPROCESS_ENV_VAR).is_ok() {
        return xargo::main_inner(xargo::XargoMode::Build);
    }

    let options = parse_cmdline();

    // Ensure there is no `Xargo.toml` file already.
    match fs::metadata("Xargo.toml") {
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => panic!("{}", e),
        Ok(_) => {
            println!("Found Xargo.toml file.");
            println!("Please remove this to coninue, as it interferes with `cargo-psp`.");

            process::exit(1);
        }
    }

    let config = PspConfig::read_from_disk();

    let xargo_toml = get_xargo_toml_text(&options);

    fs::write("Xargo.toml", xargo_toml).unwrap();

    // FIXME: This is a workaround. This should eventually be removed.
    let rustflags =
        env::var("RUSTFLAGS").unwrap_or("".into()) + " -C link-dead-code -C opt-level=3";

    // We re-initialize cargo-psp as a wrapper for Xargo, to prevent end users
    // from needing to install the `xargo` binary.
    let mut command = Command::new("cargo-psp");
    command
        .arg("build")
        .arg("--target")
        .arg(TARGET_TRIPLE)
        .args(&options.cargo_args)
        .env("RUSTFLAGS", rustflags);
    
    if let Some(src_dir) = options.local_rust_src {
        command.env("XARGO_RUST_SRC", src_dir);
    }

    if options.release {
        command.arg("--release");
    }

    let mut process = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();

    let xargo_stdout = process.stdout.take();

    // This is a pretty big hack. We wait until `xargo` starts printing and then
    // remove the toml. Then we have to manually pipe the output to our stdout.
    //
    // Ideally we could just set `XARGO_TOML_PATH` to some temporary file.
    thread::spawn(move || {
        let mut xargo_stdout = xargo_stdout.unwrap();
        let mut stdout = io::stdout();
        let mut removed_xargo_toml = false;
        let mut buf = vec![0; 8192];

        loop {
            let bytes = xargo_stdout.read(&mut buf).unwrap();

            if !removed_xargo_toml {
                fs::remove_file("Xargo.toml").unwrap();
                removed_xargo_toml = true;
            }

            if bytes == 0 {
                break;
            }

            stdout.write_all(&buf[0..bytes]).unwrap();
        }
    });

    let status = process.wait().unwrap();

    if !status.success() {
        let code = match status.code() {
            Some(i) => i,
            None => 1,
        };

        process::exit(code);
    }

    let metadata = MetadataCommand::new()
        .exec()
        .expect("failed to get cargo metadata");

    let profile_name = if options.release {
        "release"
    } else {
        "debug"
    };

    let bin_dir = metadata
        .target_directory
        .join(TARGET_TRIPLE)
        .join(profile_name);

    for id in metadata.clone().workspace_members {
        let package = metadata[&id].clone();

        for target in package.targets {
            if target.kind.iter().any(|k| k == "bin") {
                let elf_path = bin_dir.join(&target.name);
                let prx_path = bin_dir.join(target.name.clone() + ".prx");

                let sfo_path = bin_dir.join("PARAM.SFO");
                let pbp_path = bin_dir.join("EBOOT.PBP");

                Command::new("prxgen")
                    .arg(&elf_path)
                    .arg(&prx_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .expect("failed to run prxgen");

                Command::new("mksfo")
                    // Add the optional config args
                    .args(config.get_sfo_args())
                    .arg(config.title.clone().unwrap_or(target.name))
                    .arg(&sfo_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .expect("failed to run mksfo");

                Command::new("pack-pbp")
                    .arg(&pbp_path)
                    .arg(&sfo_path)
                    .arg(config.xmb_icon_png.clone().unwrap_or("NULL".into()))
                    .arg(config.xmb_icon_pmf.clone().unwrap_or("NULL".into()))
                    .arg(config.xmb_background_png.clone().unwrap_or("NULL".into()))
                    .arg(
                        config
                            .xmb_background_overlay_png
                            .clone()
                            .unwrap_or("NULL".into()),
                    )
                    .arg(config.xmb_music_at3.clone().unwrap_or("NULL".into()))
                    .arg(&prx_path)
                    .arg(config.psar.clone().unwrap_or("NULL".into()))
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .expect("failed to run pack-pbp");
            }
        }
    }
}
