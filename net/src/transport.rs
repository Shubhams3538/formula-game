use crate::{
    now,
    websocket::{WsClient, WsServer},
    Endpoint, PROTOCOL_ID,
};
use renet::{RenetClient, RenetServer};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport};
use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

pub(crate) enum ClientTransport {
    Udp(Box<NetcodeClientTransport>),
    WebSocket(WsClient),
}
impl ClientTransport {
    pub fn connect(endpoint: Endpoint, id: u64) -> Result<Self, String> {
        match endpoint {
            Endpoint::Udp(address) => {
                let bind = if address.is_ipv4() {
                    "0.0.0.0:0"
                } else {
                    "[::]:0"
                };
                let socket = UdpSocket::bind(bind).map_err(|e| e.to_string())?;
                NetcodeClientTransport::new(
                    now(),
                    ClientAuthentication::Unsecure {
                        server_addr: address,
                        client_id: id,
                        user_data: None,
                        protocol_id: PROTOCOL_ID,
                    },
                    socket,
                )
                .map(|transport| Self::Udp(Box::new(transport)))
                .map_err(|e| e.to_string())
            }
            Endpoint::WebSocket(url) => WsClient::connect(url, id).map(Self::WebSocket),
        }
    }
    pub fn update(&mut self, dt: Duration, net: &mut RenetClient) -> Result<(), String> {
        match self {
            Self::Udp(transport) => transport.update(dt, net).map_err(|e| e.to_string()),
            Self::WebSocket(transport) => transport.update(net),
        }
    }
    pub fn send_packets(&mut self, net: &mut RenetClient) -> Result<(), String> {
        match self {
            Self::Udp(transport) => transport.send_packets(net).map_err(|e| e.to_string()),
            Self::WebSocket(transport) => transport.send_packets(net),
        }
    }
    pub fn time_since_last_received_packet(&self) -> Duration {
        match self {
            Self::Udp(transport) => transport.time_since_last_received_packet(),
            Self::WebSocket(transport) => transport.elapsed(),
        }
    }
    pub fn disconnect(&mut self) {
        match self {
            Self::Udp(transport) => transport.disconnect(),
            Self::WebSocket(transport) => transport.disconnect(),
        }
    }
}

pub(crate) enum ServerTransport {
    Udp(Box<NetcodeServerTransport>),
    WebSocket(WsServer),
}
impl ServerTransport {
    pub fn address(&self) -> SocketAddr {
        match self {
            Self::Udp(transport) => transport.addresses()[0],
            Self::WebSocket(transport) => transport.address,
        }
    }
    pub fn update(&mut self, dt: Duration, net: &mut RenetServer) -> Result<(), String> {
        match self {
            Self::Udp(transport) => transport.update(dt, net).map_err(|e| e.to_string()),
            Self::WebSocket(transport) => {
                transport.update(net);
                Ok(())
            }
        }
    }
    pub fn send_packets(&mut self, net: &mut RenetServer) {
        match self {
            Self::Udp(transport) => transport.send_packets(net),
            Self::WebSocket(transport) => transport.send_packets(net),
        }
    }
    pub fn time_since_last_received_packet(&self, id: u64) -> Option<Duration> {
        match self {
            Self::Udp(transport) => transport.time_since_last_received_packet(id),
            Self::WebSocket(transport) => transport.elapsed(id),
        }
    }
    pub fn disconnect_all(&mut self, net: &mut RenetServer) {
        match self {
            Self::Udp(transport) => transport.disconnect_all(net),
            Self::WebSocket(transport) => transport.disconnect_all(net),
        }
    }
}
