/*
Definition of all core hooks. Currently only 1 file, but saved in folder structured module because I will probably
split this into multiple files later, with one containing all the Framework-level hooks and another containing all
the HostMgr level hooks.
*/

use librustneedle::{
    Hook,
    HostMgr,
    Framework,
    Module,
    PQueueOpt,
    PFilterOpt
};

pub fn load_into(mgr: &mut Framework) -> Result<(), ()> {
    let core_hooks = vec![
        // framework level
        ("modules", Hook::Framework(modules)),
        ("hooks", Hook::Framework(hooks)),
        
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

type HookRet = Result<Option<(Module, PQueueOpt, PFilterOpt)>, String>;

/*
Framework level core hook definitions
*/

fn modules(args: &[&str], fwk: &Framework) -> HookRet {
    if args.len() > 0 {
        println!("[$] args ignored");
    }

    println!("[*] Active modules:");

    for name in fwk.modules().keys() {
        println!(" * {}", name);
    }

    println!("");
    Ok(None)
}

fn hooks(args: &[&str], fwk: &Framework) -> HookRet {
    if args.len() > 0 {
        println!("[$] args ignored");
    }

    for name in fwk.hooks().keys() {
        println!(" * {}", name);
    }

    println!("");
    Ok(None)
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
        if args[1] == "all" {
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

    println!("");
    Ok(None)
}