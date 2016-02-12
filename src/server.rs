use std::thread;
use std::net::{self, SocketAddr};
use std::sync::mpsc::{channel, Sender, Receiver};
use mio::{self, Handler, EventLoop, EventLoopConfig, Token, EventSet, PollOpt};
use mio::tcp::*;
use mio::udp::*;
use message::{RobotMessage, DsMessage};

const TCP: Token = Token(0);
const UDP: Token = Token(1);

pub enum Channel {
    Tcp,
    Udp
}

macro_rules! mtry {
    ($x:expr) => {
        match $x {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err);
                return;
            }
        }
    }
}

pub struct ServerHandle {
    rx_message: Receiver<RobotMessage>,
    rx_connected: Receiver<bool>,
    tx_message: mio::Sender<(Channel, DsMessage)>,
    pub connected: bool,
}

enum ServerState {
    Disconnected,
    Connected(TcpStream, SocketAddr),
}

struct Server {
    state: ServerState,
    udp: UdpSocket,
    udp_messages: Vec<Vec<u8>>,
    tx_message: Sender<RobotMessage>,
    tx_connected: Sender<bool>,
}

impl ServerHandle {
    pub fn new() -> ServerHandle {
        let (tx_message, rx_message) = channel();
        let (tx_connected, rx_connected) = channel();

        let mut config = EventLoopConfig::new();
        config.timer_tick_ms(10);
        let mut event_loop = EventLoop::configured(config).unwrap();
        let mut server = Server::new(&mut event_loop, tx_message, tx_connected);
        let tx_ds_message = event_loop.channel();
        thread::spawn(move || {
            event_loop.timeout_ms((), 100).unwrap();
            event_loop.run(&mut server).unwrap();
        });
        ServerHandle {
            rx_message: rx_message,
            rx_connected: rx_connected,
            tx_message: tx_ds_message,
            connected: false
        }
    }

    pub fn send_udp(&self, message: DsMessage) {
        self.tx_message.send((Channel::Udp, message));
    }

    pub fn recv(&self) -> Option<RobotMessage> {
        match self.rx_message.try_recv() {
            Ok(msg) => Some(msg),
            Err(_) => None
        }
    }

    pub fn tick(&mut self) {
        while let Ok(connected) = self.rx_connected.try_recv() {
            self.connected = connected;
        }
    }
}

impl Server {
    pub fn new(event_loop: &mut EventLoop<Server>,
               tx_message: Sender<RobotMessage>,
               tx_connected: Sender<bool>) -> Server
    {
        let udp = UdpSocket::bound(&"0.0.0.0:1235".parse().unwrap()).unwrap();
        event_loop.register(&udp, UDP, EventSet::readable(), PollOpt::edge());
        Server {
            state: ServerState::Disconnected,
            udp: udp,
            udp_messages: Vec::new(),
            tx_message: tx_message,
            tx_connected: tx_connected,
        }
    }
}

impl Handler for Server {
    type Timeout = ();
    type Message = (Channel, DsMessage);

    fn ready(&mut self, event_loop: &mut EventLoop<Server>, token: Token, events: EventSet) {
        match token {
            TCP => (),
            UDP => if events.is_readable() {
                let mut buffer = [0; 64];
                if let Some((num_bytes, _)) = mtry!(self.udp.recv_from(&mut buffer)) {
                    let buffer = &buffer[..num_bytes];
                    if let Some(msg) = RobotMessage::decode(buffer) {
                        self.tx_message.send(msg).unwrap();
                    }
                }
            }
            else if events.is_writable() {
                let host_udp = match self.state {
                    ServerState::Connected(_, host_udp) => host_udp,
                    ServerState::Disconnected => {
                        println!("Got writable udp without connection");
                        return;
                    }
                };
                self.udp.send_to(&self.udp_messages.pop().unwrap()[..], &host_udp);
                if self.udp_messages.len() == 0 {
                    event_loop.reregister(&self.udp, UDP,
                                          EventSet::readable(), PollOpt::edge());
                }
            },
            Token(x) => panic!("Unknown token: {}", x)
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Server>, msg: (Channel, DsMessage)) {
        match msg.0 {
            Channel::Tcp => (),
            Channel::Udp => {
                event_loop.reregister(&self.udp, UDP,
                                      EventSet::readable() | EventSet::writable(),
                                      PollOpt::edge());
                self.udp_messages.push(msg.1.encode());
            }
        }
    }

    fn timeout(&mut self, event_loop: &mut EventLoop<Server>, timeout: ()) {
        event_loop.timeout_ms((), 100).unwrap();
        if let ServerState::Disconnected = self.state {
            let host = "10.36.36.21:1234".parse().unwrap();
            let host_udp = "10.36.36.21:1235".parse().unwrap();
            let tcp = mtry!(TcpStream::connect(&host));
            event_loop.register(&tcp, TCP, EventSet::readable(), PollOpt::edge());
            println!("Connected at {}", host);
            self.state = ServerState::Connected(tcp, host_udp);
            self.tx_connected.send(true);
        }
    }
}
