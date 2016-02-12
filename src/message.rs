use std::io::Cursor;
use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};
use sixense::ControllerData;

macro_rules! mtry {
    ($x:expr) => {
        match $x {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err);
                return None;
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum RobotMessage {
    Gyro(f64),
}

#[derive(Copy, Clone, Debug)]
pub enum Hand {
    Left,
    Right
}

#[derive(Copy, Clone, Debug)]
pub enum DsMessage {
    Sixense(ControllerData, Hand),
}

impl RobotMessage {
    pub fn decode(data: &[u8]) -> Option<RobotMessage> {
        let mut msg = Cursor::new(data);
        let token = mtry!(msg.read_u8());
        match token {
            0 => Some(RobotMessage::Gyro(mtry!(msg.read_f64::<BigEndian>()))),
            t => {
                println!("Unkown token: {}", t);
                None
            }
        }
    }
}

impl DsMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut msg = Vec::new();
        match self {
            &DsMessage::Sixense(data, hand) => {
                msg.write_u8(0);
                msg.write_f64::<BigEndian>(data.pos.x);
                msg.write_f64::<BigEndian>(data.pos.y);
                msg.write_f64::<BigEndian>(data.pos.z);
                msg.write_f64::<BigEndian>(data.rot_quat.quat().w);
                msg.write_f64::<BigEndian>(data.rot_quat.quat().i);
                msg.write_f64::<BigEndian>(data.rot_quat.quat().j);
                msg.write_f64::<BigEndian>(data.rot_quat.quat().k);
                msg.write_f64::<BigEndian>(data.joystick.x);
                msg.write_f64::<BigEndian>(data.joystick.y);
                msg.write_f64::<BigEndian>(data.trigger);
                msg.write_u8(match hand {
                    Hand::Left => 0,
                    Hand::Right => 1
                });
            },
        }
        msg
    }
}
