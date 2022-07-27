#![no_std]

extern crate alloc;
#[macro_use]
extern crate terminal_print;
extern crate getopts;

use alloc::string::String;
use alloc::vec::Vec;
use getopts::Options;

pub fn main(args: Vec<String>) -> isize {
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(_f) => {
            println!("{}", _f);
            print_usage(opts);
            return -1;
        }
    };

    if matches.opt_present("h") {
        print_usage(opts);
        return 0;
    }

    println!("Not implemented.\nArguments: {:?}", args);

    0
}

fn print_usage(opts: Options) {
    println!("{}", opts.usage(USAGE));
}

const USAGE: &'static str = "Usage: ip [ OPTIONS ] OBJECT { COMMAND | help }
where OBJECT := { link | address }
      OPTIONS := { -h[elp] }";
