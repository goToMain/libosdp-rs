use anyhow::Context;
use build_target::Os;
use std::{
    borrow::BorrowMut,
    path::{Path, PathBuf},
    process::Command,
};
type Result<T> = anyhow::Result<T, anyhow::Error>;

const OSDP_EXPORT_CONTENT: &str = "/* Auto generated from build.rs */
#ifndef OSDP_EXPORT_H_
#define OSDP_EXPORT_H_

#define OSDP_EXPORT
#define OSDP_NO_EXPORT
#define OSDP_DEPRECATED_EXPORT

#endif /* OSDP_EXPORT_H_ */
";

fn path_join(root: &str, path: &str) -> String {
    Path::new(root)
        .join(path)
        .into_os_string()
        .into_string()
        .unwrap()
}

fn configure_file(path: &str, transforms: Vec<(&str, &str)>) -> Result<()> {
    let mut contents = std::fs::read_to_string(path)?;

    for (from, to) in transforms {
        if let Some(start) = contents.find(format!("@{from}@").as_str()) {
            let range = start..start + from.len() + 2;
            contents.replace_range(range, to);
        }
    }
    std::fs::write(path, contents)?;
    Ok(())
}

fn exec_cmd(cmd: Vec<&str>) -> Result<String> {
    let mut c = Command::new(cmd[0]);
    let mut c = c.borrow_mut();
    for arg in &cmd[1..] {
        c = c.arg(*arg);
    }
    let stdout = String::from_utf8_lossy(&c.output()?.stdout).into_owned();
    Ok(stdout.trim().to_owned())
}

struct GitInfo {
    branch: String,
    tag: String,
    diff: String,
    rev: String,
    root: String,
}

impl GitInfo {
    pub fn new() -> Result<Self> {
        let diff = match exec_cmd(vec!["git", "diff", "--quiet", "--exit-code"]) {
            Ok(_) => "",
            Err(_) => "+",
        };
        Ok(GitInfo {
            branch: exec_cmd(vec!["git", "rev-parse", "--abbrev-ref", "HEAD"])?,
            tag: exec_cmd(vec!["git", "describe", "--exact-match", "--tags"])
                .unwrap_or("".to_owned()),
            diff: diff.to_owned(),
            rev: exec_cmd(vec!["git", "log", "--pretty=format:'%h'", "-n", "1"])?,
            root: exec_cmd(vec!["git", "rev-parse", "--show-toplevel"])?,
        })
    }
}

fn generate_osdp_build_headers(out_dir: &str) -> Result<()> {
    /* generate osdp_export.h */
    std::fs::write(path_join(out_dir, "osdp_export.h"), OSDP_EXPORT_CONTENT)
        .context("Failed to create osdp_export.h")?;

    /* generate osdp_config.h */
    let git = GitInfo::new()?;
    let src = "vendor/src/osdp_config.h.in";
    let dest = path_join(out_dir, "osdp_config.h");
    std::fs::copy(src, &dest).context(format!("Failed: copy {src} -> {dest}"))?;
    configure_file(
        &dest,
        vec![
            ("PROJECT_VERSION", env!("CARGO_PKG_VERSION")),
            (
                "PROJECT_NAME",
                format!("{}-rust", env!("CARGO_PKG_NAME")).as_str(),
            ),
            ("GIT_BRANCH", git.branch.as_str()),
            ("GIT_REV", git.rev.as_ref()),
            ("GIT_TAG", git.tag.as_ref()),
            ("GIT_DIFF", git.diff.as_ref()),
            ("REPO_ROOT", git.root.as_ref()),
        ],
    )
}

fn main() -> Result<()> {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    generate_osdp_build_headers(&out_dir)?;

    /* build LibOSDP */

    let mut build = cc::Build::new();
    let mut build = build
        .include("vendor/src")
        .include("vendor/include")
        .include("vendor/utils/include")
        .warnings(true)
        .include(&out_dir);

    if std::env::var("WIN_WERROR").is_err() && Os::target().unwrap() != Os::Windows {
        // TODO: Windows builds warn about various things which are legitimate
        // in other platforms. Over time, we need to assess each case and
        // handle it the way Windows likes us to do them and then remove this.
        build = build.warnings_into_errors(true)
    }

    let source_files = vec![
        "vendor/utils/src/list.c",
        "vendor/utils/src/queue.c",
        "vendor/utils/src/slab.c",
        "vendor/utils/src/utils.c",
        "vendor/utils/src/logger.c",
        "vendor/utils/src/disjoint_set.c",
        "vendor/src/osdp_common.c",
        "vendor/src/osdp_phy.c",
        "vendor/src/osdp_sc.c",
        "vendor/src/osdp_file.c",
        "vendor/src/osdp_pd.c",
        "vendor/src/osdp_cp.c",
        "vendor/src/crypto/tinyaes_src.c",
        "vendor/src/crypto/tinyaes.c",
    ];

    for file in source_files {
        build = build.file(file);
    }

    if cfg!(feature = "skip_mark_byte") {
        build = build.define("CONFIG_OSDP_SKIP_MARK_BYTE", "1");
    }

    if cfg!(feature = "packet_trace") {
        build = build
            .define("CONFIG_OSDP_PACKET_TRACE", "1")
            .file("vendor/utils/src/pcap_gen.c")
            .file("vendor/src/osdp_pcap.c");
    }

    if cfg!(feature = "data_trace") {
        build = build
            .define("CONFIG_OSDP_DATA_TRACE", "1")
            .file("vendor/utils/src/pcap_gen.c")
            .file("vendor/src/osdp_pcap.c");
    }

    let short_enums = build.get_compiler().is_like_gnu() || build.get_compiler().is_like_clang();
    if short_enums {
        build.flag("-fshort-enums");
    }
    build.compile("libosdp.a");

    /* generate bindings */

    let mut args = vec![format!("-I{}", &out_dir)];
    if short_enums {
        args.push("-fshort-enums".to_owned());
    } else {
        args.push("-fno-short-enums".to_owned());
    }
    let bindings = bindgen::Builder::default()
        .use_core()
        .header("vendor/include/osdp.h")
        .clang_args(args)
        .generate()
        .context("Unable to generate bindings")?;

    let out_path = PathBuf::from(out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .context("Couldn't write bindings!")
}
