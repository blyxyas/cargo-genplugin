use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    process::Command,
};

use syn::{Signature, __private::ToTokens, punctuated::Punctuated, token::Comma, FnArg, Item};

use clap::Parser;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
	Plugin(Args)
}

#[derive(clap::Args)]
struct Args {
    /// Path to Cargo project containing the plugin
    input: String,
    #[arg(short, default_value = "stubs")]
    /// Path for where the stub project is generated.
    stubs: String,
    #[arg(long, default_value = "false")]
    /// Toggle formatting using rustfmt
    fmt: bool,
    /// SO Object path. Example: 'my_project.so'
    so: String,
}

mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let CargoCli::Plugin(mut args) = CargoCli::parse();

    if !args.so.ends_with(".so") {
        args.so.push_str(".so");
    }

	dbg!(&args.input);

    let ast = parser::parse_file(&format!("{}/src/lib.rs", args.input))?;

    // Create Cargo project
    if !Path::new(&args.stubs).exists() {
        Command::new("cargo")
            .args(&["new", &args.stubs, "--lib"])
            .status()
            .expect("Couldn't create a new cargo project");
    };

    let f = File::create(format!("{}/src/lib.rs", args.stubs))?;
    std::fs::write(
        "stubs/Cargo.toml",
        "[package]
name = \"stubs\"
version = \"0.1.0\"
edition = \"2021\"
[dependencies]
libloading = \"0.7.4\"
lazy_static = \"1.4.0\"
",
    )?;

    let mut buf = BufWriter::new(f);
    buf.write_all(
        format!(
            "// File autogenerated by cargo-genplugin, you can use the `--fmt` flag to generate formatted files. Do not manually edit, it will be overwritten. Report issues @ github.com/blyxyas/cargo-genplugin. The following functions were generated:

			/*
			{}*/
use libloading;
			use	lazy_static;
				lazy_static::lazy_static! {{
					static ref LIB: libloading::Library =
						unsafe {{ libloading::Library::new(\"{}/target/release/{}\").expect(\"Couldn't load library {}\") }};
				}}
			\n",
            get_funcs(&ast.items), std::fs::canonicalize(&args.input).expect(&format!("Couldn't canonicalize path '{}'", args.input)).display(), args.so, args.so
        )
        .as_bytes(),
    )
    .expect("Couldn't write to buffer");

    for item in ast.items {
        if let Some(sig) = parser::parse_to_fn_sig(&item) {
            let adapted = adapt_sig(sig);
            let ts = sig.output.to_token_stream().to_string();
            let output = if ts.is_empty() {
                "()"
            } else {
                &ts[3..] /* Remove '-> ' */
            };
            buf.write_all(
                format!(
                    "pub unsafe extern fn {}({})-> {} {{\
					let func:libloading::Symbol<unsafe extern {}>=LIB.get(b\"{}\").expect(\"Couldn't load function '{}'\");\
					return func({});}}\n",
                    sig.ident,
                    sig.inputs.to_token_stream().to_string(),
                    output,
                    adapted,
                    sig.ident,
					sig.ident,
					only_pats(&sig.inputs)
                )
                .as_bytes(),
            )
            .expect("Couldn't write to buffer");
        }
    }

    buf.flush().expect("Couldn't flush");

    if args.fmt {
        Command::new("rustfmt")
            .args(&["--config", "force_explicit_abi=false", "src/lib.rs"])
            .current_dir(args.stubs)
            .status()
            .expect("Oh no!");
    }

    Ok(())
}

fn adapt_sig(sig: &Signature) -> String {
    format!(
        "fn({}) {}",
        sig.inputs.to_token_stream(),
        sig.output.to_token_stream()
    )
}

fn only_pats(punct: &Punctuated<FnArg, Comma>) -> String {
    let mut result = String::new();
    for arg in punct.iter() {
        if let FnArg::Receiver(receiver) = &arg {
            if let Some(lifetime) = receiver.lifetime() {
                result.push_str(&format!("{},", &lifetime.ident.to_string()));
            } else {
                result.push_str("self,");
            }
        } else if let FnArg::Typed(patty) = &arg {
            result.push_str(&format!("{},", &patty.pat.to_token_stream().to_string()));
        }
    }
    result
}

fn get_funcs(items: &Vec<Item>) -> String {
    let mut result = String::new();
    for item in items {
        if let Some(func) = parser::parse_to_fn(item) {
            result.push_str(&format!("* {}\n", func.sig.ident.to_string()));
        };
    }
    result
}
