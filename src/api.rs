use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use byteorder::WriteBytesExt;
use serde::Deserialize;
use tokio::{net::UdpSocket, sync::RwLock};

#[derive(Debug, Deserialize)]
struct LoginResponse {
    authentication_token: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub number_of_led: usize,
    pub measured_frame_rate: f32,
}

#[derive(Debug, Deserialize)]
pub struct VerifyResponse {
    pub code: u32,
}

#[derive(Debug, Deserialize)]
pub struct FwVersionResponse {
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub string: String,
    pub binary: Vec<u8>,
    pub last_verified: Option<Instant>,
}

impl Token {
    pub fn new(token: String) -> Result<Token> {
        let binary = base64::decode(token.clone())?;
        let token = Token {
            string: token,
            binary,
            last_verified: None,
        };

        Ok(token)
    }
}

#[derive(Clone, Default)]
pub struct TwinklyApi {
    addr: String,
    token: Arc<RwLock<Option<Token>>>,

    udp_socket: Arc<RwLock<Option<UdpSocket>>>,
    last_set_rt_mode: Arc<RwLock<Option<Instant>>>,
}

impl TwinklyApi {
    pub fn new(addr: String) -> TwinklyApi {
        TwinklyApi {
            addr,
            ..Default::default()
        }
    }

    pub async fn login(&self) -> Result<Token> {
        let addr = self.addr.clone();

        // Start by generating a new auth token
        let mut body = HashMap::new();

        // Just use a hardcoded challenge for now
        body.insert("challenge", "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=");

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{addr}/xled/v1/login"))
            .json(&body)
            .send()
            .await?
            .json::<LoginResponse>()
            .await?;

        let token = Token::new(resp.authentication_token)?;

        {
            let mut guard = self.token.write().await;
            *guard = Some(token.clone());
        }

        // Next we need to call this verify endpoint to make the token valid
        self.verify(token.clone()).await?;

        Ok(token)
    }

    pub async fn get_token(&self) -> Result<Token> {
        let token = self.token.read().await.clone();

        if let Some(token) = token {
            let valid = self.verify(token.clone()).await.is_ok();

            if !valid {
                let token = self.login().await?;
                Ok(token)
            } else {
                Ok(token)
            }
        } else {
            let token = self.login().await?;
            Ok(token)
        }
    }

    pub async fn verify(&self, token: Token) -> Result<()> {
        // Skip verify call if it has been successfully called within 1 sec
        if let Some(last_verified) = token.last_verified {
            if last_verified.elapsed() <= Duration::from_secs(10) {
                return Ok(());
            }
        }

        let mut token = token.clone();
        token.last_verified = Some(Instant::now());

        {
            let mut guard = self.token.write().await;
            *guard = Some(token.clone());
        }

        let addr = self.addr.clone();

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{addr}/xled/v1/verify"))
            .header("X-Auth-Token", token.string.clone())
            .send()
            .await?
            .json::<VerifyResponse>()
            .await?;

        if resp.code == 1000 {
            Ok(())
        } else {
            let code = resp.code;
            Err(anyhow!("verify returned error status code {code}"))
        }
    }

    pub async fn get_status(&self) -> Result<StatusResponse> {
        let addr = self.addr.clone();
        let token = self.get_token().await?;
        let client = reqwest::Client::new();

        let resp = client
            .get(format!("http://{addr}/xled/v1/gestalt"))
            .header("X-Auth-Token", token.string)
            .send()
            .await?
            .json::<StatusResponse>()
            .await?;

        Ok(resp)
    }

    pub async fn get_fw_version(&self) -> Result<String> {
        let addr = self.addr.clone();
        let client = reqwest::Client::new();

        let resp = client
            .get(format!("http://{addr}/xled/v1/fw/version"))
            .send()
            .await?
            .json::<FwVersionResponse>()
            .await?;

        Ok(resp.version)
    }

    pub async fn get_mode(&self) -> Result<String> {
        // Get device mode
        let addr = self.addr.clone();
        let token = self.get_token().await?;
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{addr}/xled/v1/led/mode"))
            .header("X-Auth-Token", token.string)
            .send()
            .await?
            .json::<HashMap<String, serde_json::Value>>()
            .await?;

        let mode: &str = resp.get("mode").unwrap().as_str().unwrap();

        Ok(mode.to_owned())
    }

    pub async fn set_mode(&self, mode: String) -> Result<()> {
        let addr = self.addr.clone();
        let token = self.get_token().await?;
        let client = reqwest::Client::new();
        let mut body = HashMap::new();
        body.insert("mode", mode);
        client
            .post(format!("http://{addr}/xled/v1/led/mode"))
            .header("X-Auth-Token", token.string)
            .json(&body)
            .send()
            .await?
            .json::<HashMap<String, serde_json::Value>>()
            .await?;

        Ok(())
    }

    pub async fn get_layout(&self) -> Result<DeviceLayout> {
        // Get device layout
        let addr = self.addr.clone();
        let token = self.get_token().await?;
        let client = reqwest::Client::new();
        let layout = client
            .get(format!("http://{addr}/xled/v1/led/layout/full"))
            .header("X-Auth-Token", token.string)
            .send()
            .await?
            .json::<DeviceLayout>()
            .await?;

        Ok(layout)
    }

    pub async fn init_udp(&self) -> Result<()> {
        let addr = self.addr.clone();
        let remote_addr: SocketAddr = format!("{addr}:7777").parse()?;

        let local_addr: SocketAddr = if remote_addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        }
        .parse()?;

        let socket = UdpSocket::bind(local_addr).await?;
        socket.connect(&remote_addr).await?;

        let socket = socket;

        {
            let mut guard = self.udp_socket.write().await;
            *guard = Some(socket);
        }

        Ok(())
    }

    pub async fn send_rt_frame(&self, frame_data: Vec<u8>) -> Result<()> {
        // Make sure we're in rt mode
        let last_set_rt_mode = *self.last_set_rt_mode.read().await;
        let should_set_rt_mode = if let Some(t) = last_set_rt_mode {
            t.elapsed() > Duration::from_secs(1)
        } else {
            true
        };

        if should_set_rt_mode {
            let api = self.clone();

            tokio::spawn(async move {
                api.set_mode(String::from("rt")).await.ok();
            });
        }

        let token_binary = self.get_token().await?.binary;
        let packet_size = 900;

        let frame_chunks = frame_data.chunks(packet_size);

        for (i, chunk) in frame_chunks.enumerate() {
            let mut packet: Vec<u8> = vec![0x03];
            packet.append(&mut token_binary.clone());
            packet.append(&mut vec![0x00, 0x00]);
            packet.write_u8(i as u8).unwrap();
            packet.append(&mut chunk.to_owned());
            {
                let guard = self.udp_socket.read().await;
                let socket = guard
                    .as_ref()
                    .context("Tried to send_rt_frame() without initialized UDP socket")?;
                socket.send(&packet).await?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Coordinates {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeviceLayout {
    pub coordinates: Vec<Coordinates>,
    pub source: String,
}

impl DeviceLayout {
    /// Normalizes all coords to be between 0.0 and 1.0
    pub fn normalized_coords(&self) -> DeviceLayout {
        DeviceLayout {
            coordinates: self
                .coordinates
                .iter()
                .map(|coord| Coordinates {
                    x: (coord.x + 1.0) / 2.0,
                    y: coord.y,
                    z: (coord.z + 1.0) / 2.0,
                })
                .collect(),
            source: self.source.clone(),
        }
    }
}
