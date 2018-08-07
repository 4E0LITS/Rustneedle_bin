use std::env::args;
use std::thread::spawn;
use std::fs::read_dir;
use std::process::exit;
use std::net::IpAddr;
use std::sync::mpsc::channel;

mod core;
mod control;
mod proc;
use proc::{
    get_arp_entry,
    get_gateway
};

extern crate librustneedle;
use librustneedle::{
    DLINKCFG,
    HostMgr,
    KnownPair,
    Framework,
};

extern crate libloading;
use libloading::Library;

extern crate pnet;
use pnet::datalink::{
    interfaces,
    channel as dlink_channel,
    Channel,
    DataLinkReceiver,
    DataLinkSender
};

use std::io::stdin;

const PLUGINS: &str = include!(concat!(env!("OUT_DIR"), "/plugin_dir"));

fn usage() {
    println!("Usage - rustneedle <interface>");
}

fn main() {
    exit(match init() {
        Err(display_usage) => {
            if display_usage {
                usage();
            }

            -1
        },

        Ok((ins, (tx, rx))) => run(ins, tx, rx)
    });
}

fn run(mut instance: Framework, channel_sender: Box<DataLinkSender>, channel_recver: Box<DataLinkReceiver>) -> i32 {
    /*
    begin tasks for datalink packet handling, then run hooks ask passed from cmdline.
    when new modules are spawned, pass packsenders and recvers to tasks as appropriate
    */
    let (recv_moddrop, recv_modqueue) = channel();
    let (packet_sender, packet_recver) = channel();
    let (recv_killer, recv_killrx) = channel();
    let (send_killer, send_killrx) = channel();

    let recv_handle = spawn(move || control::dlink_recv(channel_recver, recv_modqueue, recv_killrx));
    let send_handle = spawn(move || control::dlink_send(channel_sender, packet_recver, send_killrx));

    instance.init_task_mpscs(recv_moddrop, packet_sender);

    loop {
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
        let words: Vec<&str> = buf.trim().split(" ").collect();

        if !instance.is_running() {
            break;
        }

        // if hook resulted in new module, drop new info to tasks
        match instance.try_run_hook(words[0], &words[1..]) {
            Ok(_) => (),

            Err(e) => println!("[!] {}", e)
        };
    }

    0
}

fn init() -> Result<(Framework, (Box<DataLinkSender>, Box<DataLinkReceiver>)), bool> {
    let argv: Vec<String> = args().collect();

    if argv.len() != 2 || argv[1] == "-h" || argv[1] == "--help" {
        return Err(true);
    }

    // get interface and gateway info and mk hostmgr
    let iface = if let Some(ifc) = interfaces().into_iter().filter_map(|_ifc|
        if _ifc.name == argv[1] {
            Some(_ifc)
        } else {
            None
        }
    ).next() {
        ifc
    } else {
        println!("[!] No interface '{}' available", argv[1]);
        return Err(false);
    };

    let iface_proto = if let Some(addr) = iface.ips.iter().filter_map(|_addr| match _addr.ip() {
        IpAddr::V4(_ipv4) => Some(_ipv4),
        IpAddr::V6(_) => None
    }).next() {
        addr
    } else {
        println!("[!] No IP v4 address available for {}", argv[1]);
        return Err(false);
    };

    let gate_proto = match get_gateway(&argv[1]) {
        Ok(addr) => addr,
        Err(e) => {
            println!("[!] An error occurred fetching gateway info: {}", e);
            return Err(false);
        }
    };

    let gate_hardw = match get_arp_entry(gate_proto.to_string()) {
        Ok(addr) => addr,
        Err(e) => {
            println!("[!] An error occurred fetching gateway info: {}", e);
            return Err(false);
        }
    };

    let host_manager = HostMgr::new(KnownPair::new(iface_proto, iface.mac_address()), KnownPair::new(gate_proto, gate_hardw));

    // mk datalink channel for send/recv
    let (channel_tx, channel_rx) = match dlink_channel(&iface, DLINKCFG) {
        Ok(ch) => match ch {
            Channel::Ethernet(tx, rx) => (tx, rx),
            _ => {
                /*
                in current version of pnet, this match will never occur.
                */
                println!("[!] Error: This should never happen.");
                return Err(false);
            }
        },

        Err(e) => {
            println!("[!] An error occurred creating datalink sender/recver: {}\n(Are you root??)", e);
            return Err(false);
        }
    };

    // init framework and load core
    let mut instance = Framework::new(host_manager);

    if core::load_into(&mut instance).is_err() {
        return Err(false);
    }

    // load plugin libs and acquire hooks
    let mut libs = Vec::new();
    println!("[***] loading plugins...");

    match read_dir(PLUGINS) {
        Ok(dir) => for entry in dir {
            match entry {
                Ok(file) => match Library::new(file.path()) {
                    Ok(lib) => libs.push(lib),
                    Err(e) => println!("[!] Error loading {:#?}: {}", file.path(), e)
                },

                Err(e) => println!("[!] IO Error: {}", e)
            }
        },

        Err(e) => {
            println!("[!] Error reading {}: {}", PLUGINS, e);
            return Err(false);
        }
    };

    println!("[***] loading hooks...");

    for (n, lib) in libs.into_iter().enumerate() {
        if let Err(errs) = instance.load_hooks_from(lib) {
            println!("[!] One or more errors have occurred in lib {}:", n);

            for e in errs.into_iter() {
                println!(" * {}", e);
            }
            
            println!("");
        }
    }

    Ok((instance, (channel_tx, channel_rx)))
}