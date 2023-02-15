use std::{fs::File, io::Read};
use syn::{parse::Result, Item, Item::Fn, ItemFn, __private::ToTokens, Signature, Ident, };

pub fn parse_file(path: &str) -> Result<syn::File> {
	let mut file = File::open(path).expect(&format!("Couldn't open file '{}'", path));
	let mut content = String::new();
	file.read_to_string(&mut content).expect("Couldn't read to string");

	syn::parse_file(&content)
}

pub fn parse_to_fn_sig(item: &Item) -> Option<&Signature> {
	if let Fn(func) = item {
		return Some(&func.sig)
	}
	None
}