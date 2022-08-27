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

enum Error {
    /// An operation cannot proceed because a buffer is empty or full (the memorie for routes).
    Exhausted,
    /// An gateway is invalide.
    InvalideGateway,
    ///Interface not found
    IterNotFound(i32),
    ///No interface are configure
    NoInterface,
}

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
            Objet::Link => name == self.name() || name == "l",
            Objet::Address => name == self.name() || name == "add" || name == "a",
            Objet::Route => name == self.name() || name == "r",
        }
    }

    fn get_help(&self) -> &str {
        match *self {
            Objet::Link => "Usage: link [ show ]",
            Objet::Address => "Usage: address [ show ]",
            Objet::Route => {
                r#"Usage: route [ show | list ]
       route { add | del } ROUTE [iter <iter_num>]
ROUTE := { default | <network/mask> } via <gateway_ip>
exemple: route add default via 10.4.2.8
         route add 192.168.1.0/24 via 192.168.1.42 iter 2
         route del 192.168.1.0/24"#
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
            return 0;
        }

        return match *self {
            Objet::Link => {
                print_link();
                0
            }
            Objet::Address => {
                print_address();
                0
            }
            Objet::Route => match cmd {
                "add" => add_del_route(matches, true),
                "del" => add_del_route(matches, false),
                "" | "show" | "list" => {
                    print_route();
                    0
                }
                _ => {
                    println!("Command \"{}\" is unknown, try \"ip route help\".", cmd);
                    -1
                }
            },
        };
    }
}

fn print_link() {
    let net_iterfaces = NETWORK_INTERFACES.lock().clone();
    let mut counter = 1;
    for iterface in net_iterfaces.iter() {
        println!("interface {}:", counter);
        println!("  link/ether {}", iterface.lock().ethernet_addr());
        counter = counter + 1;
    }
}

fn print_address() {
    let net_iterfaces = NETWORK_INTERFACES.lock().clone();
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
}

fn print_route() {
    let net_iterfaces = NETWORK_INTERFACES.lock().clone();
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
}

fn add_del_route(matches: &Matches, add_route_flag: bool) -> isize {
    if add_route_flag && (matches.free.len() < 5 || matches.free[3] != "via") {
        println!("Error: invalid arguments for route add, try \"ip route help\".");
        return -1;
    }
    if !add_route_flag && (matches.free.len() < 3) {
        println!("Error: invalid arguments for route delete, try \"ip route help\".");
        return -1;
    }

    let gateway;
    if add_route_flag {
        let gateway_str = matches.free[4].as_str();
        match gateway_str.parse::<IpAddress>() {
            Ok(add) => {
                gateway = add;
            }
            Err(_) => {
                println!("Error: \"{}\" is not a valide ip gateway.", gateway_str);
                return -1;
            }
        }
    } else {
        gateway = IpAddress::v4(0, 0, 0, 0);
    }

    let cidr_str = matches.free[2].as_str();
    let cidr;
    if cidr_str == "default" {
        match gateway {
            IpAddress::Ipv4(_) => {
                cidr = IpCidr::new(IpAddress::v4(0, 0, 0, 0), 0);
            }
            IpAddress::Ipv6(_) => {
                cidr = IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0, 0), 0);
            }
            IpAddress::Unspecified | IpAddress::__Nonexhaustive => {
                println!("Error: \"{}\" is not a valide network/mask.", cidr_str);
                return -1;
            }
        }
    } else {
        match cidr_str.parse::<IpCidr>() {
            Ok(add) => {
                cidr = add;
            }
            Err(_) => {
                println!("Error: \"{}\" is not a valide network/mask.", cidr_str);
                return -1;
            }
        }
    }

    let mut iter = 0;
    let iter_arg_pos = if add_route_flag { 5 } else { 3 };
    if matches.free.len() >= iter_arg_pos + 2 {
        if matches.free[iter_arg_pos] != "iter" {
            println!("Error: when parsing iterface to change, try \"ip route help\".");
            return -1;
        }
        let iter_str = matches.free[iter_arg_pos + 1].clone();
        match iter_str.parse::<i32>() {
            Ok(i) => {
                iter = i;
            }
            Err(_) => {
                println!(
                    "Error: when parsing iterface: unknown \"{}\", try \"ip route help\".",
                    iter_str
                );
                return -1;
            }
        }
    }

    let ret_code = if add_route_flag {
        add_route(cidr, gateway, iter)
    } else {
        del_route(&cidr, iter)
    };

    match ret_code {
        Ok(_) => 0,
        Err(Error::Exhausted) => {
            println!("Error: route table capacity exhausted.");
            -1
        }
        Err(Error::InvalideGateway) => {
            println!("Error: gateway is invalid.");
            -1
        }
        Err(Error::IterNotFound(iter_not_found)) => {
            println!("Error: interface #{} was not found.", iter_not_found);
            -1
        }
        Err(Error::NoInterface) => {
            println!("Error: no interface is configured.");
            -1
        }
    }
}

fn add_route(cidr: IpCidr, gateway: IpAddress, iter: i32) -> Result<(), Error> {
    let mut counter = 0;
    let new_route;
    match gateway {
        IpAddress::Ipv4(ip4) => {
            new_route = Route::new_ipv4_gateway(ip4);
        }
        IpAddress::Ipv6(ip6) => {
            new_route = Route::new_ipv6_gateway(ip6);
        }
        IpAddress::Unspecified | IpAddress::__Nonexhaustive => {
            return Err(Error::InvalideGateway);
        }
    };

    let mut ret = Err(Error::NoInterface);
    let net_iterfaces = NETWORK_INTERFACES.lock().clone();
    for iterface in net_iterfaces.iter() {
        if counter == iter {
            let mut locked_iterface = iterface.lock();
            let routes = locked_iterface.routes_mut();
            routes.update(|route_map| match route_map.insert(cidr, new_route) {
                Ok(_) => {
                    ret = Ok(());
                }
                Err((_cidr, _route)) => {
                    ret = Err(Error::Exhausted);
                }
            });
            break;
        }
        ret = Err(Error::IterNotFound(iter));
        counter = counter + 1;
    }
    return ret;
}

fn del_route(cidr: &IpCidr, iter: i32) -> Result<(), Error> {
    let mut counter = 0;
    let net_iterfaces = NETWORK_INTERFACES.lock().clone();
    for iterface in net_iterfaces.iter() {
        if counter == iter {
            let mut locked_iterface = iterface.lock();
            let routes = locked_iterface.routes_mut();
            routes.update(|route_map| {
                route_map.remove(cidr);
            });
            return Ok(());
        }
        counter = counter + 1;
    }
    if counter == 0 {
        return Err(Error::NoInterface);
    } else {
        return Err(Error::IterNotFound(iter));
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
