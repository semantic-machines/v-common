use crate::module::module::Module;
use crate::module::ticket::Ticket;
use crate::onto::individual::Individual;
use crate::search::ft_client::FTClient;
use crate::storage::storage::{StorageId, StorageMode, VStorage};
use crate::v_api::api_client::{APIClient, IndvOp};
use crate::v_api::obj::ResultCode;
use ini::Ini;
use std::env;

pub struct Backend {
    pub storage: VStorage,
    pub fts: FTClient,
    pub api: APIClient,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::create(StorageMode::ReadOnly, false)
    }
}

impl Backend {
    pub fn create(storage_mode: StorageMode, use_remote_storage: bool) -> Self {
        let args: Vec<String> = env::args().collect();

        let conf = Ini::load_from_file("veda.properties").expect("fail load veda.properties file");
        let section = conf.section(None::<String>).expect("fail parse veda.properties");

        let mut ft_query_service_url = String::default();

        for el in args.iter() {
            if el.starts_with("--ft_query_service_url") {
                let p: Vec<&str> = el.split('=').collect();
                ft_query_service_url = p[1].to_owned();
            }
        }

        if ft_query_service_url.is_empty() {
            ft_query_service_url = section.get("ft_query_service_url").expect("param [ft_query_service_url] not found in veda.properties").to_string();
        }

        info!("use ft_query_service_url={}", ft_query_service_url);

        let storage: VStorage;

        if !use_remote_storage {
            storage = get_storage_use_prop(storage_mode);
        } else {
            let ro_storage_url = section.get("ro_storage_url").expect("param [ro_storage_url] not found in veda.properties");
            storage = VStorage::new_remote(ro_storage_url);
        }

        let ft_client = FTClient::new(ft_query_service_url);

        let param_name = "main_module_url";
        let api = if let Some(url) = Module::get_property(param_name) {
            APIClient::new(url)
        } else {
            error!("not found param {} in properties file", param_name);
            APIClient::new("".to_owned())
        };

        Backend {
            storage,
            fts: ft_client,
            api,
        }
    }

    pub fn get_sys_ticket_id(&mut self) -> Result<String, i32> {
        Module::get_sys_ticket_id_from_db(&mut self.storage)
    }

    pub fn get_literal_of_link(&mut self, indv: &mut Individual, link: &str, field: &str, to: &mut Individual) -> Option<String> {
        if let Some(v) = indv.get_literals(link) {
            for el in v {
                if self.storage.get_individual(&el, to) {
                    return to.get_first_literal(field);
                }
            }
        }
        None
    }

    pub fn get_literals_of_link(&mut self, indv: &mut Individual, link: &str, field: &str) -> Vec<String> {
        let mut res = Vec::new();
        if let Some(v) = indv.get_literals(link) {
            for el in v {
                let to = &mut Individual::default();
                if self.storage.get_individual(&el, to) {
                    if let Some(s) = to.get_first_literal(field) {
                        res.push(s);
                    }
                }
            }
        }
        res
    }

    pub fn get_datetime_of_link(&mut self, indv: &mut Individual, link: &str, field: &str, to: &mut Individual) -> Option<i64> {
        if let Some(v) = indv.get_literals(link) {
            for el in v {
                if self.storage.get_individual(&el, to) {
                    return to.get_first_datetime(field);
                }
            }
        }
        None
    }

    pub fn get_individual_h(&mut self, uri: &str) -> Option<Box<Individual>> {
        let mut iraw = Box::new(Individual::default());
        if !self.storage.get_individual(uri, &mut iraw) {
            return None;
        }
        Some(iraw)
    }

    pub fn get_individual_s(&mut self, uri: &str) -> Option<Individual> {
        let mut iraw = Individual::default();
        if !self.storage.get_individual(uri, &mut iraw) {
            return None;
        }
        Some(iraw)
    }

    pub fn get_individual<'a>(&mut self, uri: &str, iraw: &'a mut Individual) -> Option<&'a mut Individual> {
        if uri.is_empty() || !self.storage.get_individual(uri, iraw) {
            return None;
        }
        Some(iraw)
    }

    pub fn get_ticket_from_db(&mut self, id: &str) -> Ticket {
        let mut dest = Ticket::default();
        let mut indv = Individual::default();
        if self.storage.get_individual_from_db(StorageId::Tickets, id, &mut indv) {
            dest.update_from_individual(&mut indv);
            dest.result = ResultCode::Ok;
        }
        dest
    }
}

pub fn indv_apply_cmd(cmd: &IndvOp, prev_indv: &mut Individual, indv: &mut Individual) {
    if !prev_indv.is_empty() {
        let list_predicates = indv.get_predicates();

        for predicate in list_predicates {
            if predicate != "v-s:updateCounter" {
                if cmd == &IndvOp::AddTo {
                    // add value to set or ignore if exists
                    prev_indv.apply_predicate_as_add_unique(&predicate, indv);
                } else if cmd == &IndvOp::SetIn {
                    // set value to predicate
                    prev_indv.apply_predicate_as_set(&predicate, indv);
                } else if cmd == &IndvOp::RemoveFrom {
                    // remove predicate or value in set
                    prev_indv.apply_predicate_as_remove(&predicate, indv);
                }
            }
        }
    }
}

pub fn get_storage_use_prop(mode: StorageMode) -> VStorage {
    let conf = Ini::load_from_file("veda.properties").expect("fail load [veda.properties] file");
    let section = conf.section(None::<String>).expect("fail parse veda.properties");
    let tarantool_addr = if let Some(p) = section.get("tarantool_url") {
        p.to_owned()
    } else {
        warn!("param [tarantool_url] not found in veda.properties");
        "".to_owned()
    };

    let storage = if !tarantool_addr.is_empty() {
        VStorage::new_tt(tarantool_addr, "veda6", "123456")
    } else {
        VStorage::new_lmdb("./data", mode)
    };

    storage
}
