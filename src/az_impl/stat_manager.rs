use nng::{Protocol, Socket};
use std::collections::VecDeque;

pub(crate) struct StatPub {
    socket: Socket,
    url: String,
    is_connected: bool,
    message_buffer: VecDeque<String>,
}

impl StatPub {
    pub(crate) fn new(url: &str) -> Result<Self, nng::Error> {
        let socket = Socket::new(Protocol::Pub0)?;
        println!("Connected to {}", url);
        Ok(Self {
            socket,
            url: url.to_string(),
            is_connected: false,
            message_buffer: VecDeque::new(),
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

        // Объединяем все строки в одну, используя разделитель
        let combined_message = self.message_buffer.iter().map(|msg| msg.as_str()).collect::<Vec<&str>>().join("|"); // Использование '|' как разделителя; убедитесь, что он не встречается в сообщениях

        // Отправляем объединенное сообщение
        self.socket.send(combined_message.as_bytes())?;

        // Очищаем буфер после отправки
        self.message_buffer.clear();

        Ok(())
    }
}
