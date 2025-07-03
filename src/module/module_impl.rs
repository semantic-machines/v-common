use crate::module::common::sys_sig_listener;
use crate::module::info::ModuleInfo;
use crate::module::veda_backend::Backend;
use v_individual_model::onto::individual::{Individual, RawObj};
use v_individual_model::onto::parser::parse_raw;
use v_storage::{StorageId, VStorage};
use crate::v_api::api_client::IndvOp;
use crate::v_api::obj::ResultCode;
use chrono::Local;
use crossbeam_channel::{select, tick, Receiver};
use env_logger::Builder;
use ini::Ini;
use nng::options::protocol::pubsub::Subscribe;
use nng::options::Options;
use nng::options::RecvTimeout;
use nng::{Protocol, Socket};
use std::io::Write;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use std::{env, thread, time};
use v_queue::{consumer::*, record::*};

#[derive(Debug)]
#[repr(u8)]
pub enum PrepareError {
    Fatal = 101,
    Recoverable = 102,
}

const NOTIFY_CHANNEL_RECONNECT_TIMEOUT: u64 = 300;

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
                    println!("use {} = {}", p[0], v);
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
            if let Some(s) = Module::get_property("notify_channel_url") {
                notify_channel_url = s
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

    // A function that retrieves a property value from a configuration file
    // The function takes an input parameter as an argument and returns an Option<String>
    pub fn get_property<T: FromStr>(in_param: &str) -> Option<T> {
        // Load the configuration file "veda.properties" using the Ini library and panic if it fails
        let conf = Ini::load_from_file("veda.properties").expect("fail load veda.properties file");

        // Extract the [alias] section from the configuration file and panic if it fails
        let aliases = conf.section(Some("alias")).expect("fail parse veda.properties, section [alias]");

        // Collect command line arguments into a vector of strings
        let args: Vec<String> = env::args().collect();

        let params = [in_param.replace('_', "-"), in_param.replace('-', "_")];

        // Loop through the command line arguments and check if any of them match the possible parameter names
        for el in args.iter() {
            for param in &params {
                if el.starts_with(&format!("--{}", param)) {
                    // Split the argument into a key and a value
                    let p: Vec<&str> = el.split('=').collect();

                    // If the argument has a key and a value, retrieve the value and check for aliases
                    if p.len() == 2 {
                        let v = p[1].trim();
                        let val = if let Some(a) = aliases.get(v) {
                            info!("use arg --{}={}, alias={}", param, a, v);
                            a
                        } else {
                            info!("use arg --{}={}", param, v);
                            v
                        };

                        return val.parse().ok();
                    }
                }
            }
        }

        // If the parameter was not found in the command line arguments, try to retrieve it from the configuration file
        let section = conf.section(None::<String>).expect("fail parse veda.properties");

        if let Some(v) = section.get(in_param) {
            // If the parameter is found, retrieve its value and check for aliases
            let mut val = v.trim().to_owned();

            // If the value starts with a dollar sign ($), it is interpreted as an environment variable and the value of the variable is retrieved
            if val.starts_with('$') {
                if let Ok(val4var) = env::var(val.strip_prefix('$').unwrap_or_default()) {
                    info!("get env variable [{}]", val);
                    val = val4var;
                } else {
                    info!("not found env variable {}", val);
                    return None;
                }
            }

            // Check for aliases and log the parameter and its value
            let res = if let Some(a) = aliases.get(&val) {
                info!("use param [{}]={}, alias={}", in_param, a, val);
                a
            } else {
                info!("use param [{}]={}", in_param, val);
                &val
            };

            // Parse the value into the desired type and return it as an Option<T>
            return res.parse().ok();
        }

        // If the parameter was not found in the configuration file, log an error and return None
        error!("param [{}] not found", in_param);
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
        if storage.get_individual_from_db(StorageId::Tickets, "systicket", &mut indv) == ResultCode::Ok {
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

        let mut soc = None;
        let mut count_timeout_error = 0;

        let mut prev_batch_time = Instant::now();
        let update = tick(Duration::from_millis(1));
        loop {
            if let Some(qq) = &self.syssig_ch {
                select! {
                    recv(update) -> _ => {
                    }
                    recv(qq) -> _ => {
                        info!("queue {}/{}, part:{}, pos:{}", queue_consumer.queue.base_path, queue_consumer.name, queue_consumer.id, queue_consumer.count_popped);
                        info!("Exit");
                        std::process::exit (exitcode::OK);
                        //break;
                    }
                }
            }

            if let Err(PrepareError::Fatal) = heartbeat(backend, module_context) {
                error!("heartbeat: found fatal error, stop listen queue");
                break;
            }

            if soc.is_none() {
                soc = self.connect_to_notify_channel();
                if soc.is_none() {
                    thread::sleep(time::Duration::from_millis(NOTIFY_CHANNEL_RECONNECT_TIMEOUT));
                    info!("sleep {} ms", NOTIFY_CHANNEL_RECONNECT_TIMEOUT);
                }
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
                        },
                        ErrorQueue::InvalidChecksum => {
                            error!("[module] consumer:pop_body: invalid CRC, attempt seek next record");
                            queue_consumer.seek_next_pos();
                            break;
                        },
                        _ => {
                            error!("{} get msg from queue: {}", self.queue_prepared_count, e.as_str());
                            break;
                        },
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
                        },
                        Ok(b) => {
                            need_commit = b;
                        },
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
                                },
                                Ok(b) => {
                                    need_commit = b;
                                },
                            }
                        }
                    }
                }

                if need_commit {
                    queue_consumer.commit();
                }

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
                    },
                    Err(e) => {
                        if let PrepareError::Fatal = e {
                            warn!("after_batch: found fatal error, stop listen queue");
                            return;
                        }
                    },
                }
            }

            if prepared_batch_size == size_batch {
                if let Some(s) = &soc {
                    let wmsg = s.recv();
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
    init_log_with_params(module_name, filter, false);
}

pub fn init_log_with_params(module_name: &str, filter: Option<&str>, with_thread_id: bool) {
    let var_log_name = module_name.to_owned() + "_LOG";
    match std::env::var_os(&var_log_name) {
        Some(val) => println!("use env var: {}: {:?}", var_log_name, val.to_str()),
        None => std::env::set_var(&var_log_name, "info"),
    }

    let filters_str = if let Some(f) = filter {
        f.to_owned()
    } else {
        env::var(var_log_name).unwrap_or_default()
    };

    if with_thread_id {
        Builder::new()
            .format(|buf, record| {
                writeln!(buf, "{} {} [{}] - {}", thread_id::get(), Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"), record.level(), record.args())
            })
            .parse_filters(&filters_str)
            .try_init()
            .unwrap_or(())
    } else {
        Builder::new()
            .format(|buf, record| writeln!(buf, "{} [{}] - {}", Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"), record.level(), record.args()))
            .parse_filters(&filters_str)
            .try_init()
            .unwrap_or(())
    }
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
