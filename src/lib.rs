#![no_std]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate terminal_print;
extern crate getopts;
extern crate network_manager;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;
use getopts::{Matches, Options};
use network_manager::NETWORK_INTERFACES;
use smoltcp::{
    iface::Route,
    wire::{IpAddress, IpCidr},
};

struct DisplayableRoute(Route);

impl fmt::Display for DisplayableRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(time) = self.0.expires_at {
            return write!(f, "{} (expires at {})", self.0.via_router, time);
        } else {
            return write!(f, "{}", self.0.via_router);
        }
    }
}

#[derive(Debug)]
enum Objet {
    Link,
    Address,
    Route,
}

impl Objet {
    fn name(&self) -> &str {
        match *self {
            Objet::Link => "link",
            Objet::Address => "address",
            Objet::Route => "route",
        }
    }

    fn is_alias(&self, name: &str) -> bool {
        match *self {
            Objet::Link => name == "link" || name == "l",
            Objet::Address => name == "address" || name == "add" || name == "a",
            Objet::Route => name == "route" || name == "r",
        }
    }

    fn get_help(&self) -> &str {
        match *self {
            Objet::Link => "Usage: link [show]",
            Objet::Address => "Usage: address [show]",
            Objet::Route => {
                r#"Usage: route [show|list]
       route { add | del | change } ROUTE
ROUTE := NODE_SPEC [ INFO_SPEC ]"#
            }
        }
    }

    fn do_action(&self, matches: &Matches) -> isize {
        let cmd = if matches.free.len() >= 2 {
            matches.free[1].as_str()
        } else {
            ""
        };
        if cmd == "help" {
            println!("{}", self.get_help());
        }

        let net_iterfaces = NETWORK_INTERFACES.lock().clone();
        return match *self {
            Objet::Link => {
                //println!("Not implemented");
                let mut counter = 1;
                for iterface in net_iterfaces.iter() {
                    println!("interface {}:", counter);
                    println!("  link/ether {}", iterface.lock().ethernet_addr());
                    counter = counter + 1;
                }
                0
            }
            Objet::Address => {
                let mut counter = 1;
                for iterface in net_iterfaces.iter() {
                    let adds: Vec<IpCidr> = {
                        let locked_iterface = iterface.lock();
                        locked_iterface
                            .ip_addrs()
                            .iter()
                            .map(|a| a.clone())
                            .collect()
                    };
                    println!("interface {}:", counter);
                    for &add in adds.iter() {
                        match add {
                            IpCidr::Ipv4(ip4) => {
                                println!("  inet4 {}", ip4);
                            }
                            IpCidr::Ipv6(ip6) => {
                                println!("  inet6 {}", ip6);
                            }
                            IpCidr::__Nonexhaustive => {}
                        }
                    }
                    if adds.is_empty() {
                        println!("  no IP address assigned");
                    }
                    counter = counter + 1;
                }
                0
            }
            Objet::Route => {
                //println!("Not implemented");
                let mut counter = 1;
                for iterface in net_iterfaces.iter() {
                    println!("interface {}:", counter);
                    let mut routes_clone: Vec<(IpCidr, Route)> = Vec::new();
                    {
                        let mut locked_iterface = iterface.lock();
                        let routes = locked_iterface.routes_mut();
                        routes.update(|route_map| {
                            for r in route_map.iter() {
                                routes_clone.push((r.0.clone(), r.1.clone()));
                            }
                        });
                    }
                    let default_cidr_ip4 = IpCidr::new(IpAddress::v4(0, 0, 0, 0), 0);
                    let default_cidr_ip6 = IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0, 0), 0);
                    for (ip_cidr, by) in routes_clone.iter() {
                        if *ip_cidr == default_cidr_ip4 {
                            println!("  default via {}", DisplayableRoute(by.clone()));
                        }
                        if *ip_cidr == default_cidr_ip6 {
                            println!("  default via {}", DisplayableRoute(by.clone()));
                        }
                    }
                    for (ip_cidr, by) in routes_clone.iter() {
                        if *ip_cidr != default_cidr_ip4 && *ip_cidr != default_cidr_ip6 {
                            println!("  {} -> {}", ip_cidr, DisplayableRoute(by.clone()));
                        }
                    }
                    counter = counter + 1;
                }
                0
            }
        };
    }
}

impl FromStr for Objet {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Objet::Link.is_alias(s) {
            return Ok(Objet::Link);
        }
        if Objet::Address.is_alias(s) {
            return Ok(Objet::Address);
        }
        if Objet::Route.is_alias(s) {
            return Ok(Objet::Route);
        }
        return Err(format!("'{}' is not a valid value for WSType", s));
    }
}

pub fn main(args: Vec<String>) -> isize {
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f);
            print_usage_short();
            return -1;
        }
    };

    if matches.opt_present("h") {
        print_usage_long(opts);
        return 0;
    }

    if matches.free.len() == 0 {
        print_usage_short();
        return 0;
    }

    let obj_str = matches.free[0].as_str();
    let obj_res = Objet::from_str(obj_str);

    if let Err(_str) = obj_res {
        println!("Object \"{}\" is unknown, try \"ip help\"", obj_str);
        return -1;
    }

    return obj_res.unwrap().do_action(&matches);
}

fn print_usage_long(opts: Options) {
    println!("{}", opts.usage(USAGE));
}

fn print_usage_short() {
    println!("{}", USAGE);
}

const USAGE: &'static str = r#"Usage: ip [ OPTIONS ] OBJECT { COMMAND | help }
where OBJECT := { link | address | route }
      OPTIONS := { -h[elp] }"#;
