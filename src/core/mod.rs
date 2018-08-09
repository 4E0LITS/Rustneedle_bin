/*
Definition of all core hooks. Currently only 1 file, but saved in folder structured module because I will probably
split this into multiple files later, with one containing all the Framework-level hooks and another containing all
the HostMgr level hooks.
*/

use std::thread::spawn;
use std::time::Duration;
use std::sync::mpsc::{
    channel,
    Sender,
    Receiver
};

use librustneedle::{
    BROADCAST,
    Hook,
    HostMgr,
    Framework,
    Module,
    KnownPair
};

use pnet::{
    datalink::MacAddr,
    packet::{
        Packet,
        MutablePacket,
        arp::{
            ArpPacket,
            MutableArpPacket,
            ArpOperations as ArpOp,
        },
        ethernet::{
            EthernetPacket,
            MutableEthernetPacket,
            EtherType,
            EtherTypes
        }
    }
};

pub fn load_into(mgr: &mut Framework) -> Result<(), ()> {
    let core_hooks = vec![
        // framework level
        ("", Hook::Framework(_noinput)),
        ("exit", Hook::Framework(exit)),
        ("kill", Hook::Framework(kill)),
        ("modules", Hook::Framework(modules)),
        ("help", Hook::Framework(help)),
        ("hostdiscov", Hook::Framework(hostdiscov)),
        ("ping", Hook::Framework(ping)),
        
        // hostmgr level
        ("list", Hook::HostMgr(list)),
    ];

    for (name, func) in core_hooks.into_iter() {
        if mgr.hook_up(name, func).is_err() {
            println!("[!] Error: the world is ending.");
            return Err(());
        }
    }

    Ok(())
}

type HookRet = Result<Option<Module>, String>;

/*
Framework level core hook definitions
*/

fn _noinput(_args: &[&str], _fwk: &mut Framework) -> HookRet {
    Ok(None)
}

fn exit(_args: &[&str], fwk: &mut Framework) -> HookRet {
    fwk.stop();
    Ok(None)
}

fn kill(args: &[&str], fwk: &mut Framework) -> HookRet {
   for name in args.into_iter() {
       if let Err(e) = fwk.try_kill(name) {
           println!("[!] {}", e);
       } else {
           println!("[*] killed {}: ", name);
       }
   }

    Ok(None)
}

fn modules(args: &[&str], fwk: &mut Framework) -> HookRet {
    if args.len() > 0 {
        println!("[$] args ignored");
    }

    println!("[*] Active modules:");

    for name in fwk.modules().keys() {
        println!(" * {}", name);
    }

    Ok(None)
}

fn help(args: &[&str], fwk: &mut Framework) -> HookRet {
    if args.len() > 0 {
        println!("[$] args ignored");
    }

    for name in fwk.hooks().keys().filter(|_n| _n.len() > 0 ) {
        println!(" * {}", name);
    }

    Ok(None)
}

fn hostdiscov(args: &[&str], fwk: &mut Framework) -> HookRet {
    if args.len() > 0 {
        println!("[$] args ignored");
    }

    let (killtx, killrx) = channel();
    let (packet_tx, packet_rx) = channel();
    let gateway = fwk.hosts().get_gateway();
    let myself = fwk.hosts().get_myself();
    let nethosts = fwk.hosts().get_nethosts();

    fwk.insert_packet_tx(packet_tx).unwrap();

    let thread = spawn(move || {
        loop {
            if killrx.try_recv().is_ok() {
                break;
            }

            while let Ok(packet) = packet_rx.recv_timeout(Duration::from_secs(1)) {
                let p_eth = EthernetPacket::new(&packet).unwrap();
                if p_eth.get_ethertype() == EtherTypes::Arp {
                    let p_arp = ArpPacket::new(p_eth.payload()).unwrap();
                    let dst_proto = p_arp.get_sender_proto_addr();
                    let src_proto = p_arp.get_sender_proto_addr();

                    let gateway = gateway.lock().unwrap();
                    let myself = myself.lock().unwrap();
                    let mut nethosts = nethosts.lock().unwrap();


                    if (dst_proto != gateway.proto) && (dst_proto != myself.proto) {
                        let src_mac = p_arp.get_sender_hw_addr();

                        if p_arp.get_operation() == ArpOp::Request {
                            if !nethosts.macs().contains_key(&dst_proto) {
                                nethosts.insert(dst_proto);
                            }
                        } else {
                            nethosts.set_host(src_proto, src_mac);
                        }

                        println!("[*] {} is at {}", src_proto, src_mac);
                    }
                }
            }
        }

        Ok(())
    });

    Ok(Some(Module::new(thread, killtx)))
}

// TODO: this is a module, still needs opt parsing for configurability.
//i want it to be able to work in the background with count+delay
fn ping(args: &[&str], fwk: &mut Framework) -> HookRet {
    let mut targets = Vec::new();

    for arg in args.into_iter() {
        match arg.parse() {
            Ok(addr) => targets.push(addr),
            Err(e) => println!("[!] '{}': invalid entry", arg)
        };
    }

    let (killtx, killrx) = channel();
    let myself = fwk.hosts().get_gateway();
    
    let packet_tx = fwk.get_packet_queue().unwrap().clone();


    let thread = spawn(move ||{
        loop {
            if killrx.try_recv().is_ok() {
                break
            }

            let myself = myself.lock().unwrap();

            for host in targets.iter() {
                let mut packet = vec![0;42];
                
                {
                    let mut p_eth = MutableEthernetPacket::new(&mut packet).unwrap();

                    p_eth.set_source(myself.hardw);
                    p_eth.set_destination(BROADCAST);
                    p_eth.set_ethertype(EtherTypes::Arp);

                    let mut p_arp = MutableArpPacket::new(p_eth.payload_mut()).unwrap();

                    p_arp.set_operation(ArpOp::Request);
                    p_arp.set_sender_proto_addr(myself.proto);
                    p_arp.set_sender_hw_addr(myself.hardw);
                    p_arp.set_target_proto_addr(*host);
                }

                packet_tx.send(packet).unwrap();
            }
        }

        Ok(())
    });

    Ok(Some(Module::new(thread, killtx)))
}

/*
HostMgr level core hook definitions
*/

fn list(args: &[&str], mgr: &mut HostMgr) -> HookRet {
    let nethosts = mgr.acquire_nethosts();
    let macs = nethosts.macs();

    if args.len() == 0 {
        for (n, host) in nethosts.hosts().into_iter().enumerate() {
            if let Some(mac) = macs[host] {
                println!(" * {0:<3}: {1:<15} {2}", n, host.to_string(), mac);
            }
        }
    } else {
        if args[0] == "all" {
            for (n, host) in nethosts.hosts().into_iter().enumerate() {
                println!(" * {0:<3}: {1:<15} {2}", n, host.to_string(), match macs[host] {
                    Some(mac) => mac.to_string(),
                    None => String::from("MAC unknown")
                });
            }
        } else {
            let l = nethosts.len();

            for word in args.into_iter() {
                if let Ok(n) = word.parse() {
                    if n < l {
                        let addr = nethosts.get(n).unwrap();
                        println!(" * {0:<3}: {1:<15} {2}", n, addr.to_string(), match macs[addr] {
                            Some(mac) => mac.to_string(),
                            None => String::from("MAC unknown")
                        });
                    } else {
                        println!("[$] {} out of range", n);
                    }
                } else {
                    println!("[$] '{}' is invalid", word);
                }
            }
        }
    }

    Ok(None)
}