use std::{net::SocketAddr, str::FromStr};

/// A direct LAN address or a WebSocket URL, including Render's public WSS URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Udp(SocketAddr),
    WebSocket(String),
}
impl From<SocketAddr> for Endpoint {
    fn from(address: SocketAddr) -> Self {
        Self::Udp(address)
    }
}
impl FromStr for Endpoint {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if let Ok(address) = value.parse::<SocketAddr>() {
            if address.port() != 0 {
                return Ok(Self::Udp(address));
            }
        }
        let uri = value
            .parse::<axum::http::Uri>()
            .map_err(|_| endpoint_help())?;
        if !matches!(uri.scheme_str(), Some("ws" | "wss"))
            || uri.host().is_none_or(str::is_empty)
            || uri.authority().is_some_and(|a| a.as_str().contains('@'))
            || uri.query().is_some()
            || value.contains('#')
            || !matches!(uri.path(), "" | "/" | "/ws")
        {
            return Err(endpoint_help());
        }
        let scheme = uri.scheme_str().unwrap();
        Ok(Self::WebSocket(format!(
            "{scheme}://{}/ws",
            uri.authority().unwrap()
        )))
    }
}
fn endpoint_help() -> String {
    "Enter IP:port for LAN, or wss://your-server.onrender.com for online play".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_lan_and_canonicalizes_render_urls() {
        assert!(matches!(
            "192.168.1.20:7777".parse::<Endpoint>().unwrap(),
            Endpoint::Udp(_)
        ));
        assert!(matches!(
            "[::1]:7777".parse::<Endpoint>().unwrap(),
            Endpoint::Udp(_)
        ));
        for input in [
            "wss://race.onrender.com",
            "wss://race.onrender.com/",
            " wss://race.onrender.com/ws ",
        ] {
            assert_eq!(
                input.parse::<Endpoint>().unwrap(),
                Endpoint::WebSocket("wss://race.onrender.com/ws".into())
            );
        }
        for input in [
            "https://race.onrender.com",
            "wss:///ws",
            "ws://host:80/nope",
            "wss://u:p@host/ws",
            "wss://host/ws?token=a",
            "wss://host/#x",
            "127.0.0.1:0",
        ] {
            assert!(input.parse::<Endpoint>().is_err(), "{input}");
        }
    }
}
