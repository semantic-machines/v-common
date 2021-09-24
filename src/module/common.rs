use crossbeam_channel::{bounded, Receiver};
use signal_hook::consts::signal::*;
use signal_hook::consts::{SIGCONT, SIGHUP, SIGTSTP, SIGWINCH};
use std::collections::HashSet;
use std::io::Error;
use std::os::raw::c_int;
use std::thread;
#[cfg(feature = "extended-siginfo")]
type Signals = signal_hook::iterator::SignalsInfo<signal_hook::iterator::exfiltrator::origin::WithOrigin>;
use crate::onto::individual::Individual;
use crate::onto::onto::Onto;
use crate::onto::onto_index::OntoIndex;
use crate::storage::async_storage::{get_individual_from_db, AStorage};
use crate::storage::storage::VStorage;
use crate::v_api::obj::ResultCode;
use chrono::Utc;
#[cfg(not(feature = "extended-siginfo"))]
use signal_hook::iterator::Signals;
use signal_hook::low_level;
use v_queue::consumer::Consumer;
use v_queue::record::Mode;

pub const DATA_BASE_PATH: &str = "./data";

pub async fn c_load_onto(storage: &AStorage, onto: &mut Onto) -> bool {
    let onto_index = OntoIndex::load();

    info!("load {} onto elements", onto_index.len());

    for id in onto_index.data.keys() {
        if let Ok((mut indv, res)) = get_individual_from_db(&id, "", storage, None).await {
            if res == ResultCode::Ok {
                onto.update(&mut indv);
            }
        }
    }

    info!("add to hierarchy {} elements", onto.relations.len());

    let keys: Vec<String> = onto.relations.iter().map(|(key, _)| key.clone()).collect();

    for el in keys.iter() {
        let mut buf: HashSet<String> = HashSet::new();
        onto.get_subs(el, &mut buf);
        if !buf.is_empty() {
            onto.update_subs(el, &mut buf);
            //info!("{}, subs={:?}", el, buf);
        }
    }

    info!("end update subs");

    true
}

pub fn load_onto(storage: &mut VStorage, onto: &mut Onto) -> bool {
    let onto_index = OntoIndex::load();

    info!("load {} onto elements", onto_index.len());

    for id in onto_index.data.keys() {
        let mut indv: Individual = Individual::default();
        if storage.get_individual(&id, &mut indv) {
            onto.update(&mut indv);
        }
    }

    info!("add to hierarchy {} elements", onto.relations.len());

    let keys: Vec<String> = onto.relations.iter().map(|(key, _)| key.clone()).collect();

    for el in keys.iter() {
        let mut buf: HashSet<String> = HashSet::new();
        onto.get_subs(el, &mut buf);
        if !buf.is_empty() {
            onto.update_subs(el, &mut buf);
            //info!("{}, subs={:?}", el, buf);
        }
    }

    info!("end update subs");

    true
}

const SIGNALS: &[c_int] = &[SIGTERM, SIGQUIT, SIGINT, SIGTSTP, SIGWINCH, SIGHUP, SIGCHLD, SIGCONT];

pub fn sys_sig_listener() -> Result<Receiver<i32>, Error> {
    let (sender, receiver) = bounded(1);
    thread::spawn(move || {
        info!("Start system signal listener");
        let mut sigs = Signals::new(SIGNALS).unwrap();
        for signal in &mut sigs {
            warn!("Received signal {:?}", signal);
            #[cfg(feature = "extended-siginfo")]
            let signal = signal.signal;

            if signal != SIGTERM {
                low_level::emulate_default_handler(signal).unwrap();
            }

            let _ = sender.send(signal);
        }
    });

    Ok(receiver)
}

const MAIN_QUEUE_NAME: &str = "individuals-flow";

pub fn get_queue_status(id: &str) -> Individual {
    let mut out_indv = Individual::default();
    if let Some(consumer_name) = id.strip_prefix("srv:queue-state-") {
        let base_path: &str = &(DATA_BASE_PATH.to_owned() + "/queue");
        if let Ok(mut c) = Consumer::new_with_mode(base_path, consumer_name, MAIN_QUEUE_NAME, Mode::Read) {
            c.open(false);
            c.get_info();
            if c.queue.get_info_of_part(c.id, false).is_ok() {
                out_indv.set_id(id);
                out_indv.add_uri("rdf:type", "v-s:AppInfo");
                out_indv.add_datetime("v-s:created", Utc::now().naive_utc().timestamp());
                out_indv.add_uri("srv:queue", &("srv:".to_owned() + consumer_name));
                out_indv.add_integer("srv:total_count", c.queue.count_pushed as i64);
                out_indv.add_integer("srv:current_count", c.count_popped as i64);
            }
        }
    }
    out_indv
}
