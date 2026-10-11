// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! WebSocket transport used to reach telemetry servers.
//!
//! Endpoints are multiaddresses of the form `/<ip4|ip6|dns|dns4|dns6>/<host>/tcp/<port>/ws` or
//! `/wss` (or the path-carrying `x-parity-ws`/`x-parity-wss` forms, and `/tls/ws`), which is
//! what [`multiaddr::from_url`] produces for `ws://` and `wss://` URLs. Server certificates are
//! checked against the Mozilla root set shipped in `webpki-roots`, the same store the libp2p
//! WebSocket transport used before.

use futures::{future::BoxFuture, prelude::*};
use multiaddr::{Multiaddr, Protocol};
use std::{
	fmt, io,
	net::{IpAddr, SocketAddr},
	pin::Pin,
	sync::Arc,
	time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, Connector};

/// Timeout after which a connection attempt is considered failed. Includes name resolution, TLS
/// and the WebSocket HTTP upgrade.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// A trait that implements `Stream` and `Sink`.
pub(crate) trait StreamAndSink<I>: Stream + Sink<I> {}
impl<T: ?Sized + Stream + Sink<I>, I> StreamAndSink<I> for T {}

/// An established connection to a telemetry server. Each item sent is one binary WebSocket
/// message; each item received is the payload of one data message.
pub(crate) type WsStream = Pin<
	Box<dyn StreamAndSink<Vec<u8>, Item = Result<Vec<u8>, io::Error>, Error = io::Error> + Send>,
>;

/// A pending connection attempt.
pub(crate) type WsDial = BoxFuture<'static, Result<WsStream, io::Error>>;

/// Dials telemetry servers over WebSocket, with or without TLS.
#[derive(Clone)]
pub(crate) struct WsTransport {
	tls: Arc<rustls::ClientConfig>,
}

impl fmt::Debug for WsTransport {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WsTransport").finish_non_exhaustive()
	}
}

impl WsTransport {
	/// Create the transport.
	pub(crate) fn new() -> Result<Self, io::Error> {
		let mut roots = rustls::RootCertStore::empty();
		roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

		let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
			rustls::crypto::ring::default_provider(),
		))
		.with_safe_default_protocol_versions()
		.map_err(io::Error::other)?
		.with_root_certificates(roots)
		.with_no_client_auth();

		Ok(Self { tls: Arc::new(tls) })
	}

	/// Start connecting to `addr`.
	///
	/// Fails immediately if `addr` is not a WebSocket address.
	pub(crate) fn dial(&self, addr: Multiaddr) -> Result<WsDial, io::Error> {
		let target = WsTarget::parse(&addr)?;
		let connector = Connector::Rustls(Arc::clone(&self.tls));

		Ok(async move {
			tokio::time::timeout(CONNECT_TIMEOUT, target.connect(connector))
				.await
				.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))?
		}
		.boxed())
	}
}

/// Which addresses a DNS name may resolve to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
	Any,
	V4,
	V6,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Host {
	Ip(IpAddr),
	Dns(String, Family),
}

/// Where and how to open a WebSocket connection.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WsTarget {
	host: Host,
	port: u16,
	path: String,
	tls: bool,
}

impl WsTarget {
	/// Interpret a multiaddress the way the libp2p WebSocket transport did.
	fn parse(addr: &Multiaddr) -> Result<Self, io::Error> {
		let invalid = || {
			io::Error::new(io::ErrorKind::InvalidInput, format!("not a WebSocket address: {addr}"))
		};

		// The encapsulating protocol must be TCP/IP, possibly via DNS.
		let mut protocols = addr.iter();
		let mut ip = protocols.next();
		let mut tcp = protocols.next();
		let (host, port) = loop {
			match (ip, tcp) {
				(Some(Protocol::Ip4(ip)), Some(Protocol::Tcp(port))) => {
					break (Host::Ip(ip.into()), port)
				},
				(Some(Protocol::Ip6(ip)), Some(Protocol::Tcp(port))) => {
					break (Host::Ip(ip.into()), port)
				},
				(Some(Protocol::Dns(h)), Some(Protocol::Tcp(port)))
				| (Some(Protocol::Dnsaddr(h)), Some(Protocol::Tcp(port))) => {
					break (Host::Dns(h.into_owned(), Family::Any), port)
				},
				(Some(Protocol::Dns4(h)), Some(Protocol::Tcp(port))) => {
					break (Host::Dns(h.into_owned(), Family::V4), port)
				},
				(Some(Protocol::Dns6(h)), Some(Protocol::Tcp(port))) => {
					break (Host::Dns(h.into_owned(), Family::V6), port)
				},
				(Some(_), Some(p)) => {
					ip = Some(p);
					tcp = protocols.next();
				},
				_ => return Err(invalid()),
			}
		};

		// The WebSocket protocol is at the end, possibly followed by the remote's peer id.
		let mut protocols = addr.clone();
		let (tls, path) = loop {
			match protocols.pop() {
				Some(Protocol::P2p(_)) => {},
				Some(Protocol::Ws(path)) => match protocols.pop() {
					Some(Protocol::Tls) => break (true, path.into_owned()),
					Some(_) => break (false, path.into_owned()),
					None => return Err(invalid()),
				},
				Some(Protocol::Wss(path)) => break (true, path.into_owned()),
				_ => return Err(invalid()),
			}
		};

		Ok(Self { host, port, path, tls })
	}

	/// The URL of the WebSocket handshake. The host carries the port, as libp2p's did.
	fn url(&self) -> String {
		let host = match &self.host {
			Host::Ip(IpAddr::V4(ip)) => ip.to_string(),
			Host::Ip(IpAddr::V6(ip)) => format!("[{ip}]"),
			Host::Dns(name, _) => name.clone(),
		};
		let scheme = if self.tls { "wss" } else { "ws" };
		let path =
			if self.path.starts_with('/') { self.path.clone() } else { format!("/{}", self.path) };

		format!("{scheme}://{host}:{}{path}", self.port)
	}

	/// Resolve the host and try each address in turn until one accepts the TCP connection.
	async fn connect_tcp(&self) -> Result<TcpStream, io::Error> {
		let addresses: Vec<SocketAddr> = match &self.host {
			Host::Ip(ip) => vec![SocketAddr::new(*ip, self.port)],
			Host::Dns(name, family) => tokio::net::lookup_host((name.as_str(), self.port))
				.await?
				.filter(|address| match family {
					Family::Any => true,
					Family::V4 => address.is_ipv4(),
					Family::V6 => address.is_ipv6(),
				})
				.collect(),
		};

		let mut last_error =
			io::Error::new(io::ErrorKind::NotFound, "the host resolved to no usable address");
		for address in addresses {
			match TcpStream::connect(address).await {
				Ok(stream) => return Ok(stream),
				Err(error) => last_error = error,
			}
		}

		Err(last_error)
	}

	async fn connect(self, connector: Connector) -> Result<WsStream, io::Error> {
		let tcp = self.connect_tcp().await?;
		let (ws, _response) =
			tokio_tungstenite::client_async_tls_with_config(self.url(), tcp, None, Some(connector))
				.await
				.map_err(io::Error::other)?;

		let ws = ws
			.sink_map_err(io::Error::other)
			.with(|item: Vec<u8>| future::ready(Ok::<_, io::Error>(Message::Binary(item.into()))))
			.map_err(io::Error::other)
			.try_filter_map(|message| {
				future::ready(Ok(match message {
					Message::Binary(_) | Message::Text(_) => Some(message.into_data().to_vec()),
					_ => None,
				}))
			});

		Ok(Box::pin(ws))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn target(addr: &str) -> WsTarget {
		WsTarget::parse(&addr.parse().unwrap()).unwrap()
	}

	#[test]
	fn urls_become_the_same_targets_as_before() {
		let wss = multiaddr::from_url("wss://telemetry.example.org/submit/").unwrap();
		let wss = WsTarget::parse(&wss).unwrap();
		assert_eq!(
			wss,
			WsTarget {
				host: Host::Dns("telemetry.example.org".into(), Family::Any),
				port: 443,
				path: "/submit/".into(),
				tls: true,
			}
		);
		assert_eq!(wss.url(), "wss://telemetry.example.org:443/submit/");

		let ws = multiaddr::from_url("ws://127.0.0.1:8001/submit").unwrap();
		assert_eq!(WsTarget::parse(&ws).unwrap().url(), "ws://127.0.0.1:8001/submit");
	}

	#[test]
	fn multiaddr_forms() {
		assert_eq!(target("/ip6/::1/tcp/80/ws").url(), "ws://[::1]:80/");
		assert_eq!(target("/dns4/example.org/tcp/443/tls/ws").url(), "wss://example.org:443/");
		assert_eq!(
			target("/dns6/example.org/tcp/443/wss").host,
			Host::Dns("example.org".into(), Family::V6)
		);
		assert_eq!(
			target("/dns/example.org/tcp/443/x-parity-wss/%2Fsubmit%2F").url(),
			"wss://example.org:443/submit/"
		);
		assert_eq!(
			target(
				"/ip4/1.2.3.4/tcp/80/ws/p2p/12D3KooWKjsWsr4ok7DC6jwzFMyVxGLN6imqFTcJWNGHuDcMDuSf"
			)
			.url(),
			"ws://1.2.3.4:80/"
		);
	}

	#[test]
	fn non_websocket_addresses_are_rejected() {
		for addr in ["/ip4/80.123.90.4/tcp/5432", "/dns/example.org/udp/443/ws", "/memory/1"] {
			assert!(WsTarget::parse(&addr.parse().unwrap()).is_err(), "{addr}");
		}
	}

	#[tokio::test]
	async fn sends_binary_messages_over_plain_websocket() {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let port = listener.local_addr().unwrap().port();

		let server = tokio::spawn(async move {
			let (stream, _) = listener.accept().await.unwrap();
			let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
			let received = ws.next().await.unwrap().unwrap();
			ws.send(Message::Text("ack".into())).await.unwrap();
			received
		});

		let transport = WsTransport::new().unwrap();
		let mut connection = transport
			.dial(format!("/ip4/127.0.0.1/tcp/{port}/ws").parse().unwrap())
			.unwrap()
			.await
			.unwrap();
		connection.send(b"{\"msg\":\"system.connected\"}".to_vec()).await.unwrap();

		assert_eq!(
			server.await.unwrap(),
			Message::Binary(b"{\"msg\":\"system.connected\"}".to_vec().into())
		);
		assert_eq!(connection.next().await.unwrap().unwrap(), b"ack".to_vec());
	}
}
