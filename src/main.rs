use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use syn::{Ident, Signature, __private::ToTokens, ItemFn};

mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let ast = parser::parse_file("wawa/src/lib.rs")?;

    let f;
    if !Path::new("stubs").exists() {
        dbg!("X");
        fs::create_dir("stubs")?;
        f = File::create("stubs/wawa.rs")?;
    } else {
        if !Path::new("stubs/wawa.rs").exists() {
            f = File::create("stubs/wawa.rs")?;
        } else {
			std::fs::remove_file("stubs/wawa.rs")?;
			f = File::create("stubs/wawa.rs")?;
        }
    }

    let mut buf = BufWriter::new(f);
    buf.write_all(
        b"use libloading;
	fn main() {
		 
	}
	fn call_dynamic() -> Result<u32, Box<dyn std::error::Error>> {
		unsafe {
			let lib = libloading::Library::new(\"wawa/target/debug/libwawa.so\")?;
			",
    )
    .expect("Couldn't write to buffer");

    for item in ast.items {
        if let Some(sig) = parser::parse_to_fn_sig(&item) {
            buf.write_all(
                format!(
                    "\nlet func: libloading::Symbol<{}> = lib.get(b\"{}\")?;\n",
                    adapt_sig(sig),
                    sig.ident
                )
                .as_bytes(),
            )
            .expect("Couldn't write to buffer");
        }
    }

    buf.write_all(
        b"Ok(func())
	}
}
	",
    )
    .expect("Couldn't write to buffer");

	buf.flush().expect("Couldn't flush");

    Ok(())
}

fn adapt_sig(sig: &Signature) -> String {
	format!("unsafe extern fn({}) {}", sig.inputs.to_token_stream(), sig.output.to_token_stream())
}