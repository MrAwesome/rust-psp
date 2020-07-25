use super::cmdline_parsing::CargoPspOptions;

pub(crate) fn get_xargo_toml_text(options: &CargoPspOptions) -> String {
    let libc_patch = match &options.local_libc {
        Some(path) => get_libc_patch_text(&path),
        None => "".into(),
    };

    let std_build = if options.std {
        get_std_build_text()
    } else {
        "".into()
    };

    format!(
        r#"
[target.mipsel-sony-psp.dependencies.core]
stage = 0

[target.mipsel-sony-psp.dependencies.alloc]
stage = 1

[target.mipsel-sony-psp.dependencies.panic_unwind]
stage = 2

{std_build}

{libc_patch}
"#,
        std_build = std_build,
        libc_patch = libc_patch
    )
}

fn get_libc_patch_text(path: &str) -> String {
    format!(
        r#"
[patch.crates-io.libc]
path = "{path}"
"#,
        path = path
    )
}

fn get_std_build_text() -> String {
    r#"
[target.mipsel-sony-psp.dependencies.std]
stage = 4
"#
    .into()
}
