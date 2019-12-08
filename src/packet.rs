use std::io;
use std::vec::Vec;
use std::string::{String, FromUtf8Error};
use num::FromPrimitive;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug)]
pub enum PacketError {
    PacketError(String),
    IoError(io::Error),
    Utf8Error(FromUtf8Error),
}

impl From<io::Error> for PacketError {
    fn from(error: io::Error) -> Self {
        PacketError::IoError(error)
    }
}

impl From<FromUtf8Error> for PacketError {
    fn from(error: FromUtf8Error) -> Self {
        PacketError::Utf8Error(error)
    }
}

type PacketResult<T> = Result<T, PacketError>;

#[derive(Debug,PartialEq)]
pub enum Packet {
    ReqDevList,
    RepDevList(RepDevList),
    ReqImport(ReqImport),
    RepImport(RepImport),
    CmdSubmit(CmdSubmit),
    RetSubmit(RetSubmit),
    CmdUnlink(CmdUnlink),
    RetUnlink(RetUnlink)
}

#[derive(Debug,PartialEq)]
pub struct RepDevList {
    pub status: u32,
    pub num_devices: u32,
    pub devices: Vec<DeviceDescriptor>
}

#[derive(Debug,PartialEq)]
pub struct DeviceDescriptor {
    pub path: String,
    pub busid: String,
    pub busnum: u32,
    pub devnum: u32,
    pub speed: u32,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub configuration_value: u8,
    pub num_configurations: u8,
    pub num_interfaces: u8,
    pub interfaces: Vec<InterfaceDescriptor>
}

#[derive(Debug,PartialEq)]
pub struct InterfaceDescriptor {
    pub interface_class: u8,
    pub interface_subclass: u8,
    pub interface_protocol: u8
}

#[derive(Debug,PartialEq)]
pub struct ReqImport {
    pub busid: String
}

#[derive(Debug,PartialEq)]
pub struct RepImport {
    pub status: u32,
    pub path: String,
    pub busid: String,
    pub busnum: u32,
    pub devnum: u32,
    pub speed: u32,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub configuration_value: u8,
    pub num_configurations: u8,
    pub num_interfaces: u8
}

#[derive(Debug,PartialEq)]
pub struct CmdSubmit {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: Direction,
    pub ep: u32,
    pub transfer_flags: TransferFlags,
    pub buffer_length: u32,
    pub start_frame: u32,
    pub num_packets: u32,
    pub interval: u32,
    pub setup: Vec<u8>,
    pub data: Option<Vec<u8>>
}

#[derive(Debug,PartialEq)]
pub struct RetSubmit {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: Direction,
    pub ep: u32,
    pub status: u32,
    pub length: u32,
    pub start_frame: u32,
    pub num_packets: u32,
    pub error_count: u32,
    pub setup: Vec<u8>,
    pub data: Option<Vec<u8>>
}

#[derive(Debug,PartialEq)]
pub struct CmdUnlink {
    pub seq: u32,
    pub devid: u32,
    pub direction: Direction,
    pub ep: u32,
    pub seqnum: u32,
}

#[derive(Debug,PartialEq)]
pub struct RetUnlink {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: Direction,
    pub ep: u32,
    pub status: u32,
}

bitflags! {
    pub struct TransferFlags: u32 {
        const SHORT_NOT_OK = 0x001;
        const ISO_ASAP = 0x002;
        const NO_TRANSFER_DMA_MAP = 0x004;
        const NO_FSBR = 0x020;
        const ZERO_PACKET = 0x040;
        const NO_INTERRUPT = 0x080;
        const FREE_BUFFER = 0x100;
        const DIR_MASK = 0x200;
    }
}

impl TransferFlags {
    fn from_u32(val: u32) -> Result<TransferFlags, PacketError> {
        match TransferFlags::from_bits(val) {
            Some(x) => Ok(x),
            None => Err(PacketError::PacketError("Invalid transfer_flags".to_string())),
        }
    }
}

enum_from_primitive! {
    #[derive(Debug,PartialEq)]
    pub enum Direction {
        In = 0x00000001,
        Out = 0x00000000
    }
}

impl Direction {
    fn from_u32_err(val: u32) -> Result<Direction, PacketError> {
        match Direction::from_u32(val) {
            Some(x) => Ok(x),
            None => Err(PacketError::PacketError("Invalid direction value".to_string()))
        }
    }
}

enum_from_primitive! {
    #[derive(Debug,PartialEq)]
    enum PacketTypes {
        ReqDevList = 0x01118005,
        RepDevList = 0x01110005,
        ReqImport = 0x01118003,
        RepImport = 0x01110003,
        CmdSubmit = 0x00000001,
        RetSubmit = 0x00000003,
        CmdUnlink = 0x00000002,
        RetUnlink = 0x00000004,
    }
}

impl Packet {
    pub fn read(src: &mut io::Read) -> PacketResult<Packet> {
        let header = try!(src.read_u32::<BigEndian>());
        match PacketTypes::from_u32(header) {
            Some(PacketTypes::ReqDevList) => Packet::read_req_devlist(src),
            Some(PacketTypes::RepDevList) => RepDevList::read(src),
            Some(PacketTypes::ReqImport) => ReqImport::read(src),
            Some(PacketTypes::RepImport) => RepImport::read(src),
            Some(PacketTypes::CmdSubmit) => CmdSubmit::read(src),
            Some(PacketTypes::RetSubmit) => Err(PacketError::PacketError("RetSubmit not implemented".to_string())),
            Some(PacketTypes::CmdUnlink) => Err(PacketError::PacketError("CmdUnlink not implemented".to_string())),
            Some(PacketTypes::RetUnlink) => Err(PacketError::PacketError("RetUnlink not implemented".to_string())),
            None => Err(PacketError::PacketError(format!("Unknown packet header: 0x{:08x}", header).to_string()))
        }
    }

    pub fn write(&self, dst: &mut dyn io::Write) -> PacketResult<()> {
        match self {
            &Packet::ReqDevList => Packet::write_req_devlist(dst),
            &Packet::RepDevList(ref s) => s.write(dst),
            &Packet::ReqImport(ref s) => s.write(dst),
            &Packet::RepImport(ref s) => s.write(dst),
            &Packet::CmdSubmit(ref s) => s.write(dst),
            &Packet::RetSubmit(ref s) => Err(PacketError::PacketError("RetSubmit not implemented".to_string())),
            &Packet::CmdUnlink(ref s) => Err(PacketError::PacketError("CmdUnlink not implemented".to_string())),
            &Packet::RetUnlink(ref s) => Err(PacketError::PacketError("RetUnlink not implemented".to_string())),
        }    
    }

    fn read_req_devlist(src: &mut io::Read) -> PacketResult<Packet> {
        try!(src.read_u32::<BigEndian>());
        Ok(Packet::ReqDevList)
    }

    fn write_req_devlist(dst: &mut io::Write) -> PacketResult<()> {
        try!(dst.write_u32::<BigEndian>(PacketTypes::ReqDevList as u32));
        try!(dst.write_u32::<BigEndian>(0));
        Ok(())
    }    
}

impl RepDevList {
    fn read(src: &mut io::Read) -> PacketResult<Packet> {
        let status = try!(src.read_u32::<BigEndian>());
        let num_devices = try!(src.read_u32::<BigEndian>());
        let mut devices = Vec::new();
        for _ in 0..num_devices {
            let device = try!(DeviceDescriptor::read(src));
            devices.push(device);
        }
        Ok(Packet::RepDevList(RepDevList{ status, num_devices, devices }))
    }

    fn write(&self, dst: &mut io::Write) -> PacketResult<()> {
        try!(dst.write_u32::<BigEndian>(PacketTypes::RepDevList as u32));
        try!(dst.write_u32::<BigEndian>(self.status));
        try!(dst.write_u32::<BigEndian>(self.num_devices));
        for dev in &self.devices {
            try!(dev.write(dst));
        }
        Ok(())
    }
}

impl DeviceDescriptor {
    fn read(src: &mut io::Read) -> PacketResult<DeviceDescriptor> {
        let path = try!(read_fix_string(src, 256));
        let busid = try!(read_fix_string(src, 32));
        let busnum = try!(src.read_u32::<BigEndian>());
        let devnum = try!(src.read_u32::<BigEndian>());
        let speed = try!(src.read_u32::<BigEndian>());
        let id_vendor = try!(src.read_u16::<BigEndian>());
        let id_product = try!(src.read_u16::<BigEndian>());
        let bcd_device = try!(src.read_u16::<BigEndian>());
        let device_class = try!(src.read_u8());
        let device_subclass = try!(src.read_u8());
        let device_protocol = try!(src.read_u8());
        let configuration_value = try!(src.read_u8());
        let num_configurations = try!(src.read_u8());
        let num_interfaces = try!(src.read_u8());
        let mut interfaces = Vec::new();
        for _ in 0..num_interfaces {
            let interface = try!(InterfaceDescriptor::read(src));
            interfaces.push(interface);
        }
        Ok(DeviceDescriptor{
            path, busid, busnum, devnum, speed, id_vendor, id_product,
            bcd_device, device_class, device_subclass, device_protocol,
            configuration_value, num_configurations, num_interfaces, interfaces
        })
    }

    fn write(&self, dst: &mut io::Write) -> PacketResult<()> {
        try!(write_fix_string(dst, &self.path, 256));
        try!(write_fix_string(dst, &self.busid, 32));
        try!(dst.write_u32::<BigEndian>(self.busnum));
        try!(dst.write_u32::<BigEndian>(self.devnum));
        try!(dst.write_u32::<BigEndian>(self.speed));
        try!(dst.write_u16::<BigEndian>(self.id_vendor));
        try!(dst.write_u16::<BigEndian>(self.id_product));
        try!(dst.write_u16::<BigEndian>(self.bcd_device));
        try!(dst.write_u8(self.device_class));
        try!(dst.write_u8(self.device_subclass));
        try!(dst.write_u8(self.device_protocol));
        try!(dst.write_u8(self.configuration_value));
        try!(dst.write_u8(self.num_configurations));
        try!(dst.write_u8(self.num_interfaces));
        for interface in &self.interfaces {
            try!(interface.write(dst));
        }
        Ok(())
    }
}

impl InterfaceDescriptor {
    fn read(src: &mut io::Read) -> PacketResult<InterfaceDescriptor> {
        let interface_class = try!(src.read_u8());
        let interface_subclass = try!(src.read_u8());
        let interface_protocol = try!(src.read_u8());
        try!(src.read_u8());    // Padding
        Ok(InterfaceDescriptor{
            interface_class, interface_subclass, interface_protocol
        })
    }

    fn write(&self, dst: &mut io::Write) -> PacketResult<()> {
        try!(dst.write_u8(self.interface_class));
        try!(dst.write_u8(self.interface_subclass));
        try!(dst.write_u8(self.interface_protocol));
        try!(dst.write_u8(0u8));    // Padding
        Ok(())
    }
}

impl ReqImport {
    fn read(src: &mut io::Read) -> PacketResult<Packet> {
        let status = try!(src.read_u32::<BigEndian>());
        let busid = try!(read_fix_string(src, 32));
        Ok(Packet::ReqImport(ReqImport{ busid }))
    }

    fn write(&self, dst: &mut io::Write) -> PacketResult<()> {
        try!(dst.write_u32::<BigEndian>(PacketTypes::ReqImport as u32)); 
        try!(dst.write_u32::<BigEndian>(0));
        try!(write_fix_string(dst, &self.busid, 32));;
        Ok(())
    }    
}

/*    pub status: u32,
    pub path: String,
    pub busid: String,
    pub busnum: u32,
    pub devnum: u32,
    pub speed: u32,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub configuration_value: u8,
    pub num_configurations: u8,
    pub num_interfaces: u8*/
impl RepImport {
    fn read(src: &mut io::Read) -> PacketResult<Packet> {
        let status = try!(src.read_u32::<BigEndian>());
        if status != 0x0 {
            return Ok(Packet::RepImport(RepImport {
                status, path: "".to_string(), busid: "".to_string(),
                busnum: 0, devnum: 0, speed: 0, id_vendor: 0, id_product: 0,
                bcd_device: 0, device_class: 0, device_subclass: 0, device_protocol: 0,
                configuration_value: 0, num_configurations: 0, num_interfaces: 0
            }));
        }
        let path = try!(read_fix_string(src, 256));
        let busid = try!(read_fix_string(src, 32));
        let busnum = try!(src.read_u32::<BigEndian>());
        let devnum = try!(src.read_u32::<BigEndian>());
        let speed = try!(src.read_u32::<BigEndian>());
        let id_vendor = try!(src.read_u16::<BigEndian>());
        let id_product = try!(src.read_u16::<BigEndian>());
        let bcd_device = try!(src.read_u16::<BigEndian>());
        let device_class = try!(src.read_u8());
        let device_subclass = try!(src.read_u8());
        let device_protocol = try!(src.read_u8());
        let configuration_value = try!(src.read_u8());
        let num_configurations = try!(src.read_u8());
        let num_interfaces = try!(src.read_u8());
        Ok(Packet::RepImport(RepImport{ 
            status, path, busid, busnum, devnum, speed, id_vendor, id_product, bcd_device,
            device_class, device_subclass, device_protocol, configuration_value,
            num_configurations, num_interfaces
        }))
    }

    fn write(&self, dst: &mut io::Write) -> PacketResult<()> {
        try!(dst.write_u32::<BigEndian>(PacketTypes::RepImport as u32));
        try!(dst.write_u32::<BigEndian>(self.status));
        if self.status != 0 { return Ok(()) }
        try!(write_fix_string(dst, &self.path, 256));
        try!(write_fix_string(dst, &self.busid, 32));
        try!(dst.write_u32::<BigEndian>(self.busnum));
        try!(dst.write_u32::<BigEndian>(self.devnum));
        try!(dst.write_u32::<BigEndian>(self.speed));
        try!(dst.write_u16::<BigEndian>(self.id_vendor));
        try!(dst.write_u16::<BigEndian>(self.id_product));
        try!(dst.write_u16::<BigEndian>(self.bcd_device));
        try!(dst.write_u8(self.device_class));
        try!(dst.write_u8(self.device_subclass));
        try!(dst.write_u8(self.device_protocol));
        try!(dst.write_u8(self.configuration_value));
        try!(dst.write_u8(self.num_configurations));
        try!(dst.write_u8(self.num_interfaces));
        Ok(())
    }    
}

impl CmdSubmit {
    fn read(src: &mut io::Read) -> PacketResult<Packet> {
        let seqnum = try!(src.read_u32::<BigEndian>());
        println!("Seqnum: {:?}", seqnum);
        let devid = try!(src.read_u32::<BigEndian>());
        println!("Devid: {:?}", devid);
        let direction = try!(Direction::from_u32_err(try!(src.read_u32::<BigEndian>())));
        println!("Direction: {:?}", direction);
        let ep = try!(src.read_u32::<BigEndian>());
        println!("Ep: {:?}", ep);
        let transfer_flags = try!(TransferFlags::from_u32(try!(src.read_u32::<BigEndian>())));
        println!("flags: {:?}", transfer_flags);
        let buffer_length = try!(src.read_u32::<BigEndian>());
        println!("Buffer_length: {:?}", buffer_length);
        let start_frame = try!(src.read_u32::<BigEndian>());
        println!("Start_frame: {:?}", start_frame);
        let num_packets = try!(src.read_u32::<BigEndian>());
        println!("Num_Packets: {:?}", num_packets);
        let interval = try!(src.read_u32::<BigEndian>());
        println!("Interval: {:?}", interval);
        let mut setup = vec![0u8; 8];
        try!(src.read_exact(&mut setup));
        println!("Setup: {:?}", setup);
        let mut data: Option<Vec<u8>> = None;
        if direction == Direction::Out {
            let mut dv = vec![0u8; buffer_length as usize];
            src.read_exact(dv.as_mut_slice())?;
            println!("Data: {:?}", dv);
            data = Some(dv);
        }
        Ok(Packet::CmdSubmit(CmdSubmit{ 
            seqnum, devid, direction, ep, transfer_flags, buffer_length,
            start_frame, num_packets, interval, setup, data
        }))
    }

    fn write(&self, dst: &mut io::Write) -> PacketResult<()> {
        try!(dst.write_u32::<BigEndian>(PacketTypes::CmdSubmit as u32));
/*    pub seqnum: u32,
    pub devid: u32,
    pub direction: Direction,
    pub ep: u32,
    pub transfer_flags: TransferFlags,
    pub buffer_length: u32,
    pub start_frame: u32,
    pub num_packets: u32,
    pub interval: u32,
    pub setup: [u8; 8],
    pub data: Vec<u8>*/
        try!(dst.write_u32::<BigEndian>(self.seqnum));
        try!(dst.write_u32::<BigEndian>(self.devid));
        try!(dst.write_u32::<BigEndian>(self.ep));
        try!(dst.write_u32::<BigEndian>(self.transfer_flags.bits()));
        try!(dst.write_u32::<BigEndian>(self.buffer_length));
        try!(dst.write_u32::<BigEndian>(self.start_frame));
        try!(dst.write_u32::<BigEndian>(self.num_packets));
        try!(dst.write_u32::<BigEndian>(self.interval));
        try!(dst.write(&self.setup));
        if let Some(dv) = &self.data {
            dst.write(dv)?;
        }
        Ok(())
    }    
}

fn read_fix_string(src: &mut io::Read, len: usize) -> PacketResult<String> {
    let mut buf = vec![0u8; len];
    try!(src.read_exact(&mut buf));
    if !buf.is_ascii() {
        return Err(PacketError::PacketError("Read string is not ASCII".to_string()));
    }
    let len = match buf.iter().position(|&x| x == 0) {
        Some(i) => i,
        None => buf.len()
    };
    let s = try!(String::from_utf8(Vec::from(&buf[0..len])));
    Ok(s)
}

fn write_fix_string(dst: &mut io::Write, s: &str, size: usize) -> PacketResult<()> {
    if s.len() > (size-1) { // We require one 0-byte at end
        return Err(PacketError::PacketError("Write string is longer than buffer".to_string()));
    }
    if !s.is_ascii() {
        return Err(PacketError::PacketError("Write string is not ASCII".to_string()));
    }
    try!(dst.write_all(s.as_bytes()));
    if s.len() < size {
        let padding = vec![0u8; size-s.len()];
        try!(dst.write_all(&padding));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use packet::{Packet, PacketResult, read_fix_string, write_fix_string, RepDevList,
                 DeviceDescriptor, InterfaceDescriptor, ReqImport, RepImport};

    #[test]
    fn test_read_fix_string() {
        let data1 : Vec<u8>= vec!['a' as u8 ,'b' as u8, 'c' as u8, 0, 0 ];
        assert_eq!(read_fix_string(&mut data1.as_slice(), 5).unwrap(), "abc");
        assert_eq!(read_fix_string(&mut data1.as_slice(), 3).unwrap(), "abc");
    }

    #[test]
    fn test_write_fix_string() {
        let s = "abc";
        let mut buf = Vec::with_capacity(5);
        write_fix_string(&mut buf, &s, 5).unwrap();
        assert_eq!(buf, [97, 98, 99, 0, 0])
    }

    #[test]
    fn test_req_device_list() {
        let dl = Packet::ReqDevList;
        let mut buf = Vec::new();
        dl.write(&mut buf).unwrap();
        println!("Original structure: {:?}", dl);
        println!("Encoded: {:?}", buf);
        let dec = Packet::read(&mut buf.as_slice()).unwrap();
        println!("Decoded structure: {:?}", dec);
        assert_eq!(dl, dec);
    }

    #[test]
    fn test_rep_device_list() {
        let dl = Packet::RepDevList(RepDevList {
            status: 0,
            num_devices: 1,
            devices: vec![DeviceDescriptor {
                path: "/foo/bar".to_string(),
                busid: "3-2".to_string(),
                busnum: 3,
                devnum: 2,
                speed: 2,
                id_vendor: 0xaffe,
                id_product: 0xbeef,
                bcd_device: 0x0110,
                device_class: 255,
                device_subclass: 254,
                device_protocol: 253,
                configuration_value: 1,
                num_configurations: 2,
                num_interfaces: 2,
                interfaces: vec![
                    InterfaceDescriptor {
                        interface_class: 23,
                        interface_subclass: 26,
                        interface_protocol: 29
                    }, InterfaceDescriptor {
                        interface_class: 65,
                        interface_subclass: 85,
                        interface_protocol: 2
                    }
                ]
            }]
        });
        let mut buf = Vec::new();
        dl.write(&mut buf).unwrap();
        println!("Original structure: {:?}", dl);
        println!("Encoded: {:?}", buf);
        let dec = Packet::read(&mut buf.as_slice()).unwrap();
        println!("Decoded structure: {:?}", dec);
        assert_eq!(dl, dec);
    }

    #[test]
    fn test_req_import() {
        let dl = Packet::ReqImport(ReqImport{
            busid: "3-2".to_string()
        });
        let mut buf = Vec::new();
        dl.write(&mut buf).unwrap();
        println!("Original structure: {:?}", dl);
        println!("Encoded: {:?}", buf);
        let dec = Packet::read(&mut buf.as_slice()).unwrap();
        println!("Decoded structure: {:?}", dec);
        assert_eq!(dl, dec);
    }    

    #[test]
    fn test_rep_import() {
        let dl = Packet::RepImport(RepImport {
            status: 0,
            path: "/foo/bar".to_string(),
            busid: "3-2".to_string(),
            busnum: 3,
            devnum: 2,
            speed: 2,
            id_vendor: 0xaffe,
            id_product: 0xbeef,
            bcd_device: 0x0110,
            device_class: 255,
            device_subclass: 254,
            device_protocol: 253,
            configuration_value: 1,
            num_configurations: 2,
            num_interfaces: 2,
        });
        let mut buf = Vec::new();
        dl.write(&mut buf).unwrap();
        println!("Original structure: {:?}", dl);
        println!("Encoded: {:?}", buf);
        let dec = Packet::read(&mut buf.as_slice()).unwrap();
        println!("Decoded structure: {:?}", dec);
        assert_eq!(dl, dec);
    }
}
