use crate::module::module_impl::Module;
use crate::module::ticket::Ticket;
use crate::onto::individual::Individual;
use crate::search::ft_client::FTClient;
use crate::storage::common::{StorageId, StorageMode, VStorage};
use crate::v_api::api_client::{AuthClient, IndvOp, MStorageClient};
use crate::v_api::obj::ResultCode;
use std::env;
use url::Url;

pub struct Backend {
    pub storage: VStorage,
    pub fts: FTClient,
    pub mstorage_api: MStorageClient,
    pub auth_api: AuthClient,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::create(StorageMode::ReadOnly, false)
    }
}

impl Backend {
    pub fn create(storage_mode: StorageMode, use_remote_storage: bool) -> Self {
        let args: Vec<String> = env::args().collect();

        let mut ft_query_service_url = String::default();

        for el in args.iter() {
            if el.starts_with("--ft_query_service_url") {
                let p: Vec<&str> = el.split('=').collect();
                ft_query_service_url = p[1].to_owned();
            }
        }

        if ft_query_service_url.is_empty() {
            ft_query_service_url = Module::get_property("ft_query_service_url").expect("param [ft_query_service_url] not found in veda.properties");
        }

        info!("use ft_query_service_url={}", ft_query_service_url);

        let storage: VStorage = if !use_remote_storage {
            get_storage_use_prop(storage_mode)
        } else {
            let ro_storage_url = Module::get_property("ro_storage_url").expect("param [ro_storage_url] not found in veda.properties");
            VStorage::new_remote(&ro_storage_url)
        };

        let ft_client = FTClient::new(ft_query_service_url);

        let param_name = "main_module_url";
        let mstorage_api = if let Some(url) = Module::get_property(param_name) {
            MStorageClient::new(url)
        } else {
            error!("not found param {} in properties file", param_name);
            MStorageClient::new("".to_owned())
        };

        let param_name = "auth_url";
        let auth_api = if let Some(url) = Module::get_property(param_name) {
            AuthClient::new(url)
        } else {
            error!("not found param {} in properties file", param_name);
            AuthClient::new("".to_owned())
        };

        Backend {
            storage,
            fts: ft_client,
            mstorage_api,
            auth_api,
        }
    }

    pub fn get_sys_ticket_id(&mut self) -> Result<String, i32> {
        Module::get_sys_ticket_id_from_db(&mut self.storage)
    }

    pub fn get_literal_of_link(&mut self, indv: &mut Individual, link: &str, field: &str, to: &mut Individual) -> Option<String> {
        if let Some(v) = indv.get_literals(link) {
            for el in v {
                if self.storage.get_individual(&el, to) == ResultCode::Ok {
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
                if self.storage.get_individual(&el, to) == ResultCode::Ok {
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
                if self.storage.get_individual(&el, to) == ResultCode::Ok {
                    return to.get_first_datetime(field);
                }
            }
        }
        None
    }

    pub fn get_individual_h(&mut self, uri: &str) -> Option<Box<Individual>> {
        let mut iraw = Box::<Individual>::default();
        if self.storage.get_individual(uri, &mut iraw) != ResultCode::Ok {
            return None;
        }
        Some(iraw)
    }

    pub fn get_individual_s(&mut self, uri: &str) -> Option<Individual> {
        let mut iraw = Individual::default();
        if self.storage.get_individual(uri, &mut iraw) != ResultCode::Ok {
            return None;
        }
        Some(iraw)
    }

    pub fn get_individual<'a>(&mut self, uri: &str, iraw: &'a mut Individual) -> Option<&'a mut Individual> {
        if uri.is_empty() || self.storage.get_individual(uri, iraw) != ResultCode::Ok {
            return None;
        }
        Some(iraw)
    }

    pub fn get_ticket_from_db(&mut self, id: &str) -> Ticket {
        let mut dest = Ticket::default();
        let mut indv = Individual::default();
        if self.storage.get_individual_from_db(StorageId::Tickets, id, &mut indv) == ResultCode::Ok{
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
    get_storage_with_prop(mode, "db_connection")
}

pub fn get_storage_with_prop(mode: StorageMode, prop_name: &str) -> VStorage {
    let mut lmdb_db_path = None;

    if let Some(p) = Module::get_property(prop_name) {
        if p.contains("tcp://") {
            match Url::parse(&p) {
                Ok(url) => {
                    let host = url.host_str().unwrap_or("127.0.0.1");
                    let port = url.port().unwrap_or(3309);
                    let user = url.username();
                    let pass = url.password().unwrap_or("123");
                    return VStorage::new_tt(format!("{}:{}", host, port), user, pass);
                },
                Err(e) => {
                    error!("fail parse {}, err={}", p, e);
                },
            }
        } else {
            lmdb_db_path = Some(p);
        }
    }

    if let Some(db_path) = lmdb_db_path {
        return VStorage::new_lmdb(&db_path, mode, None);
    }

    VStorage::none()
}
