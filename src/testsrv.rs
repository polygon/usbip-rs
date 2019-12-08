extern crate vusbip;
extern crate bufstream;
use bufstream::BufStream;
use vusbip::packet::{Packet, PacketError, DeviceDescriptor, InterfaceDescriptor, RepDevList, RepImport, ReqImport};
use std::io::{Read,Write};
use std::net::{TcpListener, TcpStream};
use std::io;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3240").unwrap();
    println!("USBIP Testserver");
    for s in listener.incoming() {
        let mut stream = s.unwrap();
        handle_stream(stream);
    }
}

fn handle_stream(mut tcp_stream: TcpStream) {
    let mut stream = BufStream::new(tcp_stream);
    let dl = Packet::RepDevList(RepDevList {
        status: 0,
        num_devices: 1,
        devices: vec![DeviceDescriptor {
            path: "/foo/bar".to_string(),
            busid: "3-2".to_string(),
            busnum: 3,
            devnum: 2,
            speed: 2,
            id_vendor: 0x0403,
            id_product: 0x6001,
            bcd_device: 0x0110,
            device_class: 255,
            device_subclass: 0,
            device_protocol: 0,
            configuration_value: 1,
            num_configurations: 2,
            num_interfaces: 2,
            interfaces: vec![
                InterfaceDescriptor {
                    interface_class: 255,
                    interface_subclass: 26,
                    interface_protocol: 29
                }, InterfaceDescriptor {
                    interface_class: 255,
                    interface_subclass: 85,
                    interface_protocol: 2
                }
            ]
        }]
    });
    let ri = Packet::RepImport(RepImport {
        status: 0,
        path: "/foo/bar".to_string(),
        busid: "3-2".to_string(),
        busnum: 3,
        devnum: 2,
        speed: 2,
        id_vendor: 0x0403,
        id_product: 0x6001,
        bcd_device: 0x0110,
        device_class: 255,
        device_subclass: 0,
        device_protocol: 0,
        configuration_value: 1,
        num_configurations: 2,
        num_interfaces: 2,
    });
    println!("Client connected");
    loop {
        let pkt = Packet::read(&mut stream);
        println!("Received: {:?}", pkt);
        match pkt {
            Ok(Packet::ReqDevList) => { dl.write(&mut stream).unwrap(); stream.flush().unwrap() },
            Ok(Packet::ReqImport(_)) => { ri.write(&mut stream).unwrap(); stream.flush().unwrap() },
            Ok(s) => println!("Unhandled packet received: {:?}", s),
            Err(PacketError::PacketError(_)) => println!("Invalid packet received"),
            Err(_) => {println!("Error, closing connection"); return},
        }
    }
}
