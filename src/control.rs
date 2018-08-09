use std::sync::{
    Arc,
    mpsc::{
        Sender,
        Receiver
    }
};



use pnet::datalink::{
    DataLinkSender,
    DataLinkReceiver
};

pub fn dlink_recv(mut channel: Box<DataLinkReceiver>, module_queue: Receiver<Sender<Arc<Vec<u8>>>>, killer: Receiver<()>) {
    let mut modules = Vec::new();

    loop {
        if killer.try_recv().is_ok() {
            break;
        }

        while let Ok(new) = module_queue.try_recv() {
            modules.push(new);
        }

        /*
        when packet is received, pass shared ref to all modules. If a module is no longer alive,
        remove it from the list.
         */
        if let Ok(raw) = channel.next() {
            let packet = Arc::new(Vec::from(raw));
            let mut dead = Vec::new();

            for (idx, module) in modules.iter().enumerate() {
                if module.send(packet.clone()).is_err() {
                    dead.push(idx);
                }
            }

            for idx in dead.into_iter().rev(){
                modules.remove(idx);
            }
        }
    }
}

pub fn dlink_send(mut channel: Box<DataLinkSender>, packet_queue: Receiver<Vec<u8>>, killer: Receiver<()>) {
    loop {
        if killer.try_recv().is_ok() {
            break;
        }

        // recv waiting packets and send
        while let Ok(packet) = packet_queue.try_recv() {
            channel.send_to(&packet, None).unwrap().unwrap();
        }
    }
}