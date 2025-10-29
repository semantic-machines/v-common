use nng::{Protocol, Socket};
use rand::distr::Alphanumeric;
use rand::Rng;
use std::collections::VecDeque;
use std::time::Duration;

// Statistics collection mode
#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum StatMode {
    Full,
    Minimal,
    None,
}

// Statistics context wrapper
pub(crate) struct Stat {
    pub(crate) point: StatPub,
    pub(crate) mode: StatMode,
}

// Parse stat mode from string
pub(crate) fn parse_stat_mode(stat_mode_str: Option<String>) -> StatMode {
    if let Some(v) = stat_mode_str {
        match v.to_lowercase().as_str() {
            "full" => StatMode::Full,
            "minimal" => StatMode::Minimal,
            "off" => StatMode::None,
            "none" => StatMode::None,
            _ => StatMode::None,
        }
    } else {
        StatMode::None
    }
}

pub(crate) struct StatPub {
    socket: Socket,
    url: String,
    is_connected: bool,
    message_buffer: VecDeque<String>,
    sender_id: String,
    duration: Duration,
}

impl StatPub {
    pub(crate) fn new(url: &str) -> Result<Self, nng::Error> {
        let socket = Socket::new(Protocol::Pub0)?;

        let sender_id: String = rand::rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect();

        info!("StatManager: id={}, connected to {}", sender_id, url);

        Ok(Self {
            socket,
            url: url.to_string(),
            is_connected: false,
            message_buffer: VecDeque::new(),
            sender_id,
            duration: Duration::default(),
        })
    }

    fn connect(&mut self) -> Result<(), nng::Error> {
        self.socket.dial(&self.url)?;
        self.is_connected = true;
        Ok(())
    }

    pub(crate) fn collect(&mut self, message: String) {
        self.message_buffer.push_back(message);
    }

    pub(crate) fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    pub(crate) fn flush(&mut self) -> Result<(), nng::Error> {
        if !self.is_connected {
            self.connect()?;
        }

        //if self.message_buffer.is_empty() {
        //    return Ok(());
        //}

        // Combine all messages into one string using semicolon as separator
        let combined_message = self.message_buffer.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join(";");

        // Format string with date, sender ID and combined messages,
        // using comma as separator between elements
        let message_with_timestamp = format!("{},{},{}", self.sender_id, self.duration.as_micros(), combined_message);

        // Send message
        self.socket.send(message_with_timestamp.as_bytes())?;

        // Clear buffer after sending
        self.message_buffer.clear();

        Ok(())
    }
}

// Format message for statistics collection based on cache usage
pub(crate) fn format_stat_message(key: &str, use_cache: bool, from_cache: bool) -> String {
    match (use_cache, from_cache) {
        (true, true) => format!("{}/C", key),
        (true, false) => format!("{}/cB", key),
        (false, _) => format!("{}/B", key),
    }
}

