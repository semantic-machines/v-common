use crate::module::common::sys_sig_listener;
use crate::module::info::ModuleInfo;
use crate::module::ticket::Ticket;
use crate::module::veda_backend::Backend;
use crate::onto::datatype::Lang;
use crate::onto::individual::{Individual, RawObj};
use crate::onto::individual2msgpack::to_msgpack;
use crate::onto::parser::parse_raw;
use crate::storage::storage::{StorageId, VStorage};
use crate::v_api::api_client::IndvOp;
use crate::v_api::obj::ResultCode;
use chrono::{Local, NaiveDateTime, Utc};
use crossbeam_channel::{select, tick, Receiver};
use env_logger::Builder;
use ini::Ini;
use nng::options::protocol::pubsub::Subscribe;
use nng::options::Options;
use nng::options::RecvTimeout;
use nng::{Protocol, Socket};
use std::io::Write;
use std::time::Duration;
use std::time::Instant;
use std::{env, thread, time};
use uuid::Uuid;
use v_queue::{consumer::*, record::*};

#[derive(Debug)]
#[repr(u8)]
pub enum PrepareError {
    Fatal = 101,
    Recoverable = 102,
}

const TICKS_TO_UNIX_EPOCH: i64 = 62_135_596_800_000;

pub struct Module {
    pub(crate) queue_prepared_count: i64,
    notify_channel_url: String,
    pub(crate) is_ready_notify_channel: bool,
    notify_channel_read_timeout: Option<u64>,
    pub(crate) max_timeout_between_batches: Option<u64>,
    pub(crate) min_batch_size_to_cancel_timeout: Option<u32>,
    pub max_batch_size: Option<u32>,
    pub(crate) subsystem_id: Option<i64>,
    pub(crate) syssig_ch: Option<Receiver<i32>>,
    pub(crate) name: String,
    onto_types: Vec<String>,
}

impl Default for Module {
    fn default() -> Self {
        Module::create(None, "")
    }
}

impl Module {
    pub fn new_with_name(name: &str) -> Self {
        Module::create(None, name)
    }

    pub fn create(module_id: Option<i64>, module_name: &str) -> Self {
        let args: Vec<String> = env::args().collect();

        let conf = Ini::load_from_file("veda.properties").expect("fail load veda.properties file");
        let section = conf.section(None::<String>).expect("fail parse veda.properties");

        let mut notify_channel_url = String::default();
        let mut max_timeout_between_batches = None;
        let mut min_batch_size_to_cancel_timeout = None;
        let mut max_batch_size = None;
        let mut notify_channel_read_timeout = None;

        for el in args.iter() {
            if el.starts_with("--max_timeout_between_batches") {
                let p: Vec<&str> = el.split('=').collect();
                if let Ok(v) = p[1].parse::<u64>() {
                    max_timeout_between_batches = Some(v);
                    info!("use {} = {} ms", p[0], v);
                }
            } else if el.starts_with("--min_batch_size_to_cancel_timeout") {
                let p: Vec<&str> = el.split('=').collect();
                if let Ok(v) = p[1].parse::<u32>() {
                    min_batch_size_to_cancel_timeout = Some(v);
                    info!("use {} = {}", p[0], v);
                }
            } else if el.starts_with("--max_batch_size") {
                let p: Vec<&str> = el.split('=').collect();
                if let Ok(v) = p[1].parse::<u32>() {
                    max_batch_size = Some(v);
                    info!("use {} = {}", p[0], v);
                }
            } else if el.starts_with("--notify_channel_read_timeout") {
                let p: Vec<&str> = el.split('=').collect();
                if let Ok(v) = p[1].parse::<u64>() {
                    notify_channel_read_timeout = Some(v);
                    info!("use {} = {} ms", p[0], v);
                }
            } else if el.starts_with("--notify_channel_url") {
                let p: Vec<&str> = el.split('=').collect();
                notify_channel_url = p[1].to_owned();
            }
        }

        if notify_channel_url.is_empty() {
            if let Some(s) = section.get("notify_channel_url") {
                notify_channel_url = s.to_owned()
            }
        }

        let onto_types = vec![
            "rdfs:Class",
            "owl:Class",
            "rdfs:Datatype",
            "owl:Ontology",
            "rdf:Property",
            "owl:DatatypeProperty",
            "owl:ObjectProperty",
            "owl:OntologyProperty",
            "owl:AnnotationProperty",
            "v-ui:PropertySpecification",
            "v-ui:DatatypePropertySpecification",
            "v-ui:ObjectPropertySpecification",
            "v-ui:TemplateSpecification",
            "v-ui:ClassModel",
        ];

        Module {
            queue_prepared_count: 0,
            notify_channel_url,
            is_ready_notify_channel: false,
            max_timeout_between_batches,
            min_batch_size_to_cancel_timeout,
            max_batch_size,
            subsystem_id: module_id,
            notify_channel_read_timeout,
            syssig_ch: None,
            name: module_name.to_owned(),
            onto_types: onto_types.iter().map(|x| x.to_string()).collect(),
        }
    }

    pub fn new() -> Self {
        Module::create(None, "")
    }

    pub fn get_property(param: &str) -> Option<String> {
        let args: Vec<String> = env::args().collect();
        for el in args.iter() {
            if el.starts_with(&format!("--{}", param)) {
                let p: Vec<&str> = el.split('=').collect();
                if p.len() == 2 {
                    let v = p[1].trim();
                    info!("use arg --{}={}", param, v);
                    return Some(p[1].to_owned());
                }
            }
        }

        let conf = Ini::load_from_file("veda.properties").expect("fail load veda.properties file");

        let section = conf.section(None::<String>).expect("fail parse veda.properties");
        if let Some(v) = section.get(param) {
            let v = v.trim();
            info!("use param {}={}", param, v);
            return Some(v.to_string());
        }

        error!("param [{}] not found", param);
        None
    }

    pub fn is_content_onto(&self, cmd: IndvOp, new_state: &mut Individual, prev_state: &mut Individual) -> bool {
        if cmd != IndvOp::Remove {
            if new_state.any_exists_v("rdf:type", &self.onto_types) {
                return true;
            }
        } else if prev_state.any_exists_v("rdf:type", &self.onto_types) {
            return true;
        }
        false
    }

    pub fn get_sys_ticket_id_from_db(storage: &mut VStorage) -> Result<String, i32> {
        let mut indv = Individual::default();
        if storage.get_individual_from_db(StorageId::Tickets, "systicket", &mut indv) {
            if let Some(c) = indv.get_first_literal("v-s:resource") {
                return Ok(c);
            }
        }
        Err(-1)
    }

    pub(crate) fn connect_to_notify_channel(&mut self) -> Option<Socket> {
        if !self.is_ready_notify_channel && !self.notify_channel_url.is_empty() {
            let soc = Socket::new(Protocol::Sub0).unwrap();

            let timeout = if let Some(t) = self.notify_channel_read_timeout {
                t
            } else {
                1000
            };

            if let Err(e) = soc.set_opt::<RecvTimeout>(Some(Duration::from_millis(timeout))) {
                error!("fail set timeout, {} err={}", self.notify_channel_url, e);
                return None;
            }

            if let Err(e) = soc.dial(&self.notify_channel_url) {
                error!("fail connect to, {} err={}", self.notify_channel_url, e);
                return None;
            } else {
                let all_topics = vec![];
                if let Err(e) = soc.set_opt::<Subscribe>(all_topics) {
                    error!("fail subscribe, {} err={}", self.notify_channel_url, e);
                    soc.close();
                    self.is_ready_notify_channel = false;
                    return None;
                } else {
                    info!("success subscribe on queue changes: {}", self.notify_channel_url);
                    self.is_ready_notify_channel = true;
                    return Some(soc);
                }
            }
        }
        None
    }

    pub fn listen_queue_raw<T>(
        &mut self,
        queue_consumer: &mut Consumer,
        module_context: &mut T,
        before_batch: &mut fn(&mut Backend, &mut T, batch_size: u32) -> Option<u32>,
        prepare: &mut fn(&mut Backend, &mut T, &RawObj, &Consumer) -> Result<bool, PrepareError>,
        after_batch: &mut fn(&mut Backend, &mut T, prepared_batch_size: u32) -> Result<bool, PrepareError>,
        heartbeat: &mut fn(&mut Backend, &mut T) -> Result<(), PrepareError>,
        backend: &mut Backend,
    ) {
        self.listen_queue_comb(queue_consumer, module_context, before_batch, Some(prepare), None, after_batch, heartbeat, backend)
    }

    pub fn listen_queue<T>(
        &mut self,
        queue_consumer: &mut Consumer,
        module_context: &mut T,
        before_batch: &mut fn(&mut Backend, &mut T, batch_size: u32) -> Option<u32>,
        prepare: &mut fn(&mut Backend, &mut T, &mut Individual, &Consumer) -> Result<bool, PrepareError>,
        after_batch: &mut fn(&mut Backend, &mut T, prepared_batch_size: u32) -> Result<bool, PrepareError>,
        heartbeat: &mut fn(&mut Backend, &mut T) -> Result<(), PrepareError>,
        backend: &mut Backend,
    ) {
        self.listen_queue_comb(queue_consumer, module_context, before_batch, None, Some(prepare), after_batch, heartbeat, backend)
    }

    fn listen_queue_comb<T>(
        &mut self,
        queue_consumer: &mut Consumer,
        module_context: &mut T,
        before_batch: &mut fn(&mut Backend, &mut T, batch_size: u32) -> Option<u32>,
        prepare_raw: Option<&mut fn(&mut Backend, &mut T, &RawObj, &Consumer) -> Result<bool, PrepareError>>,
        prepare_indv: Option<&mut fn(&mut Backend, &mut T, &mut Individual, &Consumer) -> Result<bool, PrepareError>>,
        after_batch: &mut fn(&mut Backend, &mut T, prepared_batch_size: u32) -> Result<bool, PrepareError>,
        heartbeat: &mut fn(&mut Backend, &mut T) -> Result<(), PrepareError>,
        backend: &mut Backend,
    ) {
        if let Ok(ch) = sys_sig_listener() {
            self.syssig_ch = Some(ch);
        }

        let mut soc = Socket::new(Protocol::Sub0).unwrap();
        let mut count_timeout_error = 0;

        let mut prev_batch_time = Instant::now();
        let update = tick(Duration::from_millis(1));
        loop {
            if let Some(qq) = &self.syssig_ch {
                select! {
                    recv(update) -> _ => {
                    }
                    recv(qq) -> _ => {
                        info!("Exit");
                        std::process::exit (exitcode::OK);
                        //break;
                    }
                }
            }

            match heartbeat(backend, module_context) {
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
                if let Some(new_size) = before_batch(backend, module_context, size_batch) {
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

                if let Some(&mut f) = prepare_raw {
                    match f(backend, module_context, &raw, queue_consumer) {
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

                if let Some(&mut f) = prepare_indv {
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
                            match f(backend, module_context, &mut queue_element, queue_consumer) {
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
                match after_batch(backend, module_context, prepared_batch_size) {
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

pub fn get_inner_binobj_as_individual<'a>(queue_element: &'a mut Individual, field_name: &str, new_indv: &'a mut Individual) -> bool {
    let binobj = queue_element.get_first_binobj(field_name);
    if binobj.is_some() {
        new_indv.set_raw(&binobj.unwrap_or_default());
        if parse_raw(new_indv).is_ok() {
            return true;
        }
    }
    false
}

pub fn get_cmd(queue_element: &mut Individual) -> Option<IndvOp> {
    let wcmd = queue_element.get_first_integer("cmd");
    wcmd?;

    Some(IndvOp::from_i64(wcmd.unwrap_or_default()))
}

pub fn init_log(module_name: &str) {
    init_log_with_filter(module_name, None)
}

pub fn init_log_with_filter(module_name: &str, filter: Option<&str>) {
    let var_log_name = module_name.to_owned() + "_LOG";
    match std::env::var_os(var_log_name.to_owned()) {
        Some(val) => println!("use env var: {}: {:?}", var_log_name, val.to_str()),
        None => std::env::set_var(var_log_name.to_owned(), "info"),
    }

    let filters_str = if let Some(f) = filter {
        f.to_owned()
    } else {
        env::var(var_log_name).unwrap_or_default()
    };

    Builder::new()
        .format(|buf, record| writeln!(buf, "{} [{}] - {}", Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"), record.level(), record.args()))
        .parse_filters(&filters_str)
        .try_init()
        .unwrap_or(())
}

pub fn create_new_ticket(login: &str, user_id: &str, duration: i64, ticket: &mut Ticket, storage: &mut VStorage) {
    let mut ticket_indv = Individual::default();

    ticket.result = ResultCode::FailStore;
    ticket_indv.add_string("rdf:type", "ticket:ticket", Lang::NONE);

    if !ticket.id.is_empty() && !ticket.id.is_empty() {
        ticket_indv.set_id(&ticket.id);
    } else {
        ticket_indv.set_id(&Uuid::new_v4().to_hyphenated().to_string());
    }

    ticket_indv.add_string("ticket:login", login, Lang::NONE);
    ticket_indv.add_string("ticket:accessor", user_id, Lang::NONE);

    let now = Utc::now();
    let start_time_str = format!("{:?}", now.naive_utc());

    if start_time_str.len() > 28 {
        ticket_indv.add_string("ticket:when", &start_time_str[0..28], Lang::NONE);
    } else {
        ticket_indv.add_string("ticket:when", &start_time_str, Lang::NONE);
    }

    ticket_indv.add_string("ticket:duration", &duration.to_string(), Lang::NONE);

    let mut raw1: Vec<u8> = Vec::new();
    if to_msgpack(&ticket_indv, &mut raw1).is_ok() && storage.put_kv_raw(StorageId::Tickets, ticket_indv.get_id(), raw1) {
        ticket.update_from_individual(&mut ticket_indv);
        ticket.result = ResultCode::Ok;
        ticket.start_time = (TICKS_TO_UNIX_EPOCH + now.timestamp_millis()) * 10_000;
        ticket.end_time = ticket.start_time + duration as i64 * 10_000_000;

        let end_time_str = format!("{:?}", NaiveDateTime::from_timestamp((ticket.end_time / 10_000 - TICKS_TO_UNIX_EPOCH) / 1_000, 0));
        info!("create new ticket {}, login={}, user={}, start={}, end={}", ticket.id, ticket.user_login, ticket.user_uri, start_time_str, end_time_str);
    } else {
        error!("fail store ticket {:?}", ticket)
    }
}

pub fn create_sys_ticket(storage: &mut VStorage) -> Ticket {
    let mut ticket = Ticket::default();
    create_new_ticket("veda", "cfg:VedaSystem", 90_000_000, &mut ticket, storage);

    if ticket.result == ResultCode::Ok {
        let mut sys_ticket_link = Individual::default();
        sys_ticket_link.set_id("systicket");
        sys_ticket_link.add_uri("rdf:type", "rdfs:Resource");
        sys_ticket_link.add_uri("v-s:resource", &ticket.id);
        let mut raw1: Vec<u8> = Vec::new();
        if to_msgpack(&sys_ticket_link, &mut raw1).is_ok() && storage.put_kv_raw(StorageId::Tickets, sys_ticket_link.get_id(), raw1) {
            return ticket;
        } else {
            error!("fail store system ticket link")
        }
    } else {
        error!("fail create sys ticket")
    }

    ticket
}

pub fn get_info_of_module(module_name: &str) -> Option<(i64, i64)> {
    let module_info = ModuleInfo::new("./data", module_name, false);
    if module_info.is_err() {
        error!("fail open info of [{}], err={:?}", module_name, module_info.err());
        return None;
    }

    let mut info = module_info.unwrap();
    info.read_info()
}

pub fn wait_load_ontology() -> i64 {
    wait_module("input-onto", 1)
}

pub fn wait_module(module_name: &str, wait_op_id: i64) -> i64 {
    if wait_op_id < 0 {
        error!("wait module [{}] to complete op_id={}", module_name, wait_op_id);
        return -1;
    }

    info!("wait module [{}] to complete op_id={}", module_name, wait_op_id);
    loop {
        let module_info = ModuleInfo::new("./data", module_name, false);
        if module_info.is_err() {
            error!("fail open info of [{}], err={:?}", module_name, module_info.err());
            thread::sleep(time::Duration::from_millis(300));
            continue;
        }

        let mut info = module_info.unwrap();
        loop {
            if let Some((_, committed)) = info.read_info() {
                if committed >= wait_op_id {
                    info!("wait module [{}] to complete op_id={}, found commited_op_id={}", module_name, wait_op_id, committed);
                    return committed;
                }
            } else {
                error!("fail read info for module [{}]", module_name);
                //break;
            }
            thread::sleep(time::Duration::from_millis(300));
        }

        //break;
    }

    //-1
}
