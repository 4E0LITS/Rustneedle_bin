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

pub fn dlink_recv(channel: Box<DataLinkReceiver>, module_queue: Receiver<PackFilter>) {

}

pub fn dlink_send(channel: Box<DataLinkSender>, module_queue: Receiver<Receiver<Vec<u8>>>) {

}