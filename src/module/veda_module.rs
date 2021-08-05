use crate::module::common::sys_sig_listener;
use crate::module::module::{init_log, Module, PrepareError};
use crate::onto::individual::{Individual, RawObj};
use crate::onto::parser::parse_raw;
use crossbeam_channel::{select, tick};
use nng::{Protocol, Socket};
use std::time::{Duration, Instant};
use std::{thread, time};
use v_queue::consumer::Consumer;
use v_queue::record::ErrorQueue;

pub trait VedaQueueModule {
    fn before_batch(&mut self, size_batch: u32) -> Option<u32>;
    fn prepare(&mut self, queue_element: &mut Individual) -> Result<bool, PrepareError>;
    fn after_batch(&mut self, prepared_batch_size: u32) -> Result<bool, PrepareError>;
    fn heartbeat(&mut self) -> Result<(), PrepareError>;
    fn before_start(&mut self);
    fn before_exit(&mut self);
}

impl Module {
    pub fn prepare_queue(&mut self, veda_module: &mut dyn VedaQueueModule) {
        init_log(&self.name);

        let queue_consumer = &mut Consumer::new("./data/queue", &self.name, "individuals-flow").expect("!!!!!!!!! FAIL QUEUE");

        if let Ok(ch) = sys_sig_listener() {
            self.syssig_ch = Some(ch);
        }

        let mut soc = Socket::new(Protocol::Sub0).unwrap();
        let mut count_timeout_error = 0;

        let mut prev_batch_time = Instant::now();
        let update = tick(Duration::from_millis(1));
        veda_module.before_start();
        loop {
            if let Some(qq) = &self.syssig_ch {
                select! {
                    recv(update) -> _ => {
                    }
                    recv(qq) -> _ => {
                        info!("Exit");
                        veda_module.before_exit();
                        std::process::exit (exitcode::OK);
                        //break;
                    }
                }
            }

            match veda_module.heartbeat() {
                Err(e) => {
                    if let PrepareError::Fatal = e {
                        error!("heartbeat: found fatal error, stop listen queue");
                        break;
                    }
                }
                _ => {}
            }

            if let Some(s) = self.connect_to_notify_channel() {
                soc = s;
            }

            // read queue current part info
            if let Err(e) = queue_consumer.queue.get_info_of_part(queue_consumer.id, true) {
                error!("{} get_info_of_part {}: {}", self.queue_prepared_count, queue_consumer.id, e.as_str());
                continue;
            }

            let size_batch = queue_consumer.get_batch_size();

            let mut max_size_batch = size_batch;
            if let Some(m) = self.max_batch_size {
                max_size_batch = m;
            }

            if size_batch > 0 {
                debug!("queue: batch size={}", size_batch);
                if let Some(new_size) = veda_module.before_batch(size_batch) {
                    max_size_batch = new_size;
                }
            }

            let mut prepared_batch_size = 0;
            for _it in 0..max_size_batch {
                // пробуем взять из очереди заголовок сообщения
                if !queue_consumer.pop_header() {
                    break;
                }

                let mut raw = RawObj::new(vec![0; (queue_consumer.header.msg_length) as usize]);

                // заголовок взят успешно, занесем содержимое сообщения в структуру Individual
                if let Err(e) = queue_consumer.pop_body(&mut raw.data) {
                    match e {
                        ErrorQueue::FailReadTailMessage => {
                            break;
                        }
                        ErrorQueue::InvalidChecksum => {
                            error!("[module] consumer:pop_body: invalid CRC, attempt seek next record");
                            queue_consumer.seek_next_pos();
                            break;
                        }
                        _ => {
                            error!("{} get msg from queue: {}", self.queue_prepared_count, e.as_str());
                            break;
                        }
                    }
                }

                let mut need_commit = true;
                /*
                                if let Some(&mut f) = prepare_raw {
                                    match f(self, module, &raw, queue_consumer) {
                                        Err(e) => {
                                            if let PrepareError::Fatal = e {
                                                warn!("prepare: found fatal error, stop listen queue");
                                                return;
                                            }
                                        }
                                        Ok(b) => {
                                            need_commit = b;
                                        }
                                    }
                                }
                */
                {
                    let mut queue_element = Individual::new_raw(raw);
                    if parse_raw(&mut queue_element).is_ok() {
                        let mut is_processed = true;
                        if let Some(assigned_subsystems) = queue_element.get_first_integer("assigned_subsystems") {
                            if assigned_subsystems > 0 {
                                if let Some(my_subsystem_id) = self.subsystem_id {
                                    if assigned_subsystems & my_subsystem_id == 0 {
                                        is_processed = false;
                                    }
                                } else {
                                    is_processed = false;
                                }
                            }
                        }

                        if is_processed {
                            match veda_module.prepare(&mut queue_element) {
                                Err(e) => {
                                    if let PrepareError::Fatal = e {
                                        warn!("prepare: found fatal error, stop listen queue");
                                        return;
                                    }
                                }
                                Ok(b) => {
                                    need_commit = b;
                                }
                            }
                        }
                    }
                }

                queue_consumer.next(need_commit);

                self.queue_prepared_count += 1;

                if self.queue_prepared_count % 1000 == 0 {
                    info!("get from queue, count: {}", self.queue_prepared_count);
                }
                prepared_batch_size += 1;
            }

            if size_batch > 0 {
                match veda_module.after_batch(prepared_batch_size) {
                    Ok(b) => {
                        if b {
                            queue_consumer.commit();
                        }
                    }
                    Err(e) => {
                        if let PrepareError::Fatal = e {
                            warn!("after_batch: found fatal error, stop listen queue");
                            return;
                        }
                    }
                }
            }

            if prepared_batch_size == size_batch {
                let wmsg = soc.recv();
                if let Err(e) = wmsg {
                    debug!("fail recv from queue notify channel, err={:?}", e);

                    if count_timeout_error > 0 && size_batch > 0 {
                        warn!("queue changed but we not received notify message, need reconnect...");
                        self.is_ready_notify_channel = false;
                        count_timeout_error += 1;
                    }
                } else {
                    count_timeout_error = 0;
                }
            }

            if let Some(t) = self.max_timeout_between_batches {
                let delta = prev_batch_time.elapsed().as_millis() as u64;
                if let Some(c) = self.min_batch_size_to_cancel_timeout {
                    if prepared_batch_size < c && delta < t {
                        thread::sleep(time::Duration::from_millis(t - delta));
                        info!("sleep {} ms", t - delta);
                    }
                } else if delta < t {
                    thread::sleep(time::Duration::from_millis(t - delta));
                    info!("sleep {} ms", t - delta);
                }
            }

            prev_batch_time = Instant::now();
        }
    }
}
