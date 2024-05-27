use chrono::Utc;
use nng::{Protocol, Socket};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::collections::VecDeque;

pub(crate) struct StatPub {
    socket: Socket,
    url: String,
    is_connected: bool,
    message_buffer: VecDeque<String>,
    sender_id: String,
}

impl StatPub {
    pub(crate) fn new(url: &str) -> Result<Self, nng::Error> {
        let socket = Socket::new(Protocol::Pub0)?;

        let sender_id: String = thread_rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect();

        info!("StatManager: id={}, connected to {}", sender_id, url);

        Ok(Self {
            socket,
            url: url.to_string(),
            is_connected: false,
            message_buffer: VecDeque::new(),
            sender_id,
        })
    }

    fn connect(&mut self) -> Result<(), nng::Error> {
        self.socket.dial(&self.url)?;
        self.is_connected = true;
        Ok(())
    }

    pub(crate) fn collect(&mut self, message: &str) {
        self.message_buffer.push_back(message.to_string());
    }

    pub(crate) fn flush(&mut self) -> Result<(), nng::Error> {
        if !self.is_connected {
            self.connect()?;
        }

        if self.message_buffer.is_empty() {
            return Ok(());
        }

        // Объединяем все сообщения в одну строку, используя точку с запятой в качестве разделителя
        let combined_message = self.message_buffer.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join(";");

        // Формируем строку с датой, идентификатором отправителя и объединенными сообщениями,
        // используя запятую в качестве разделителя между элементами
        let message_with_timestamp = format!("{},{}", self.sender_id, combined_message);

        // Отправляем сообщение
        self.socket.send(message_with_timestamp.as_bytes())?;

        // Очищаем буфер после отправки
        self.message_buffer.clear();

        Ok(())
    }
}
