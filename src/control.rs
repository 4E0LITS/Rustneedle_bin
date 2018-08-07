use std::time::Duration;
use std::sync::{
    mpsc::{
        Sender,
        Receiver
    }
};

use librustneedle::PackFilter;

use pnet::datalink::{
    DataLinkSender,
    DataLinkReceiver
};

use pnet::packet::{
    MutablePacket,
    Packet,
    
};

pub fn dlink_recv(mut channel: Box<DataLinkReceiver>, module_queue: Receiver<PackFilter>, killer: Receiver<()>) {
    let mut modules = Vec::new();

    loop {
        if killer.try_recv().is_ok() {
            break;
        }

        while let Ok(new) = module_queue.try_recv() {
            modules.push(new);
        }

        if let Ok(raw) = channel.next() {
            let mut new_raw = raw.clone();

            //let mut 
        }
    }
}

pub fn dlink_send(channel: Box<DataLinkSender>, packet_queue: Receiver<Vec<u8>>, killer: Receiver<()>) {
    //let mut modules = Vec::new();

    loop {
        if killer.try_recv().is_ok() {
            break;
        }

    }
}