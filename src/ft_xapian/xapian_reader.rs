use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::ft_xapian::index_schema::IndexerSchema;
use crate::ft_xapian::init_db_path;
use crate::ft_xapian::key2slot::Key2Slot;
use crate::ft_xapian::vql::TTA;
use crate::ft_xapian::xapian_vql::{exec_xapian_query_and_queue_authorize, get_sorter, transform_vql_to_xapian, AuxContext};
use crate::module::common::load_onto;
use crate::module::info::ModuleInfo;
use crate::onto::individual::Individual;
use crate::onto::onto_impl::Onto;
use crate::onto::onto_index::OntoIndex;
use crate::search::common::{FTQuery, QueryResult};
use crate::storage::async_storage::{get_individual_from_db, AStorage};
use crate::storage::common::VStorage;
use crate::v_api::obj::{OptAuthorize, ResultCode};
use futures::executor::block_on;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::ops::Sub;
use std::time::Instant;
use std::time::SystemTime;
use xapian_rusty::*;

const MAX_WILDCARD_EXPANSION: i32 = 20_000;
const BASE_PATH: &str = "./data";

pub struct DatabaseQueryParser {
    db: Database,
    qp: QueryParser,
}

impl DatabaseQueryParser {
    fn add_database(&mut self, db_name: &str, opened_db: &mut HashMap<String, Database>) -> Result<()> {
        if let Some(add_db) = opened_db.get_mut(db_name) {
            self.db.add_database(add_db)?;
        }
        Ok(())
    }
}

pub struct XapianReader {
    pub index_schema: IndexerSchema,
    pub onto: Onto,
    pub onto_modified: SystemTime,
    using_dbqp: HashMap<Vec<String>, DatabaseQueryParser>,
    opened_db: HashMap<String, Database>,
    xapian_stemmer: Stem,
    xapian_lang: String,
    mdif: ModuleInfo,
    key2slot: Key2Slot,
    db2path: HashMap<String, String>,
    committed_op_id: i64,
    az: LmdbAzContext,
}

impl XapianReader {
    pub fn new(lang: &str, storage: &mut VStorage) -> Option<Self> {
        let indexer_module_info = ModuleInfo::new(BASE_PATH, "fulltext_indexer", true);
        if indexer_module_info.is_err() {
            error!("{:?}", indexer_module_info.err());
            return None;
        }

        let mut onto = Onto::default();
        load_onto(storage, &mut onto);

        let mut xr = XapianReader {
            using_dbqp: Default::default(),
            opened_db: Default::default(),
            xapian_stemmer: Stem::new(lang).unwrap(),
            xapian_lang: lang.to_string(),
            index_schema: Default::default(),
            mdif: indexer_module_info.unwrap(),
            key2slot: Key2Slot::load().unwrap_or_default(),
            onto,
            db2path: init_db_path(),
            committed_op_id: 0,
            onto_modified: SystemTime::now(),
            az: LmdbAzContext::default(),
        };

        xr.load_index_schema(storage);

        Some(xr)
    }

    pub fn new_without_init(lang: &str) -> Option<Self> {
        let indexer_module_info = ModuleInfo::new(BASE_PATH, "fulltext_indexer", true);
        if indexer_module_info.is_err() {
            error!("{:?}", indexer_module_info.err());
            return None;
        }

        let now = SystemTime::now();

        let xr = XapianReader {
            using_dbqp: Default::default(),
            opened_db: Default::default(),
            xapian_stemmer: Stem::new(lang).unwrap(),
            xapian_lang: lang.to_string(),
            index_schema: Default::default(),
            mdif: indexer_module_info.unwrap(),
            key2slot: Key2Slot::load().unwrap_or_default(),
            onto: Onto::default(),
            db2path: init_db_path(),
            committed_op_id: 0,
            onto_modified: now.sub(now.elapsed().unwrap_or_default()),
            az: LmdbAzContext::default(),
        };

        Some(xr)
    }

    pub fn query_use_authorize(&mut self, request: FTQuery, storage: &mut VStorage, op_auth: OptAuthorize, reopen: bool) -> QueryResult {
        if reopen {
            if let Err(e) = self.reopen_dbs() {
                error!("fail reopen xapian databases: {:?}", e);
            }
        }

        let mut res_out_list = vec![];
        fn add_out_element(id: &str, ctx: &mut Vec<String>) {
            ctx.push(id.to_owned());
        }

        if let Some(t) = OntoIndex::get_modified() {
            if t > self.onto_modified {
                load_onto(storage, &mut self.onto);
                self.onto_modified = t;
            }
        }
        if self.index_schema.is_empty() {
            self.load_index_schema(storage);
        }

        if let Ok(mut res) = block_on(self.query_use_collect_fn(&request, add_out_element, op_auth, &mut res_out_list)) {
            res.result = res_out_list;
            debug!("res={:?}", res);
            return res;
        }
        QueryResult::default()
    }

    pub fn query(&mut self, request: FTQuery, storage: &mut VStorage) -> QueryResult {
        self.query_use_authorize(request, storage, OptAuthorize::YES, false)
    }

    pub async fn query_use_collect_fn<T>(
        &mut self,
        request: &FTQuery,
        add_out_element: fn(uri: &str, ctx: &mut T),
        op_auth: OptAuthorize,
        out_list: &mut T,
    ) -> Result<QueryResult> {
        let total_time = Instant::now();
        let mut sr = QueryResult::default();

        let wtta = TTA::parse_expr(&request.query);

        if wtta.is_none() {
            error!("fail parse query (phase 1) [{}], tta is empty", request.query);
            sr.result_code = ResultCode::BadRequest;
            return Ok(sr);
        }

        if self.key2slot.is_need_reload()? {
            self.key2slot = Key2Slot::load()?;
        }

        let mut tta = wtta.unwrap();

        let db_names = self.get_dn_names(&tta, &request.databases);

        debug!("db_names={:?}", db_names);
        debug!(
            "user_uri=[{}] query=[{}] str_sort=[{}], db_names=[{:?}], from=[{}], top=[{}], limit=[{}]",
            request.user, request.query, request.sort, request.databases, request.from, request.top, request.limit
        );
        debug!("TTA [{}]", tta);

        if let Some((_, new_committed_op_id)) = self.mdif.read_info() {
            if new_committed_op_id > self.committed_op_id {
                info!("search:reopen_db: new committed_op_id={} > prev committed_op_id={}", new_committed_op_id, self.committed_op_id);
                self.reopen_dbs()?;
                self.committed_op_id = new_committed_op_id;
            } else {
                debug!("search:check reopen_db: new committed_op_id={}, prev committed_op_id={}", new_committed_op_id, self.committed_op_id);
            }
        }

        self.open_dbqp_if_need(&db_names)?;

        let mut query = Query::new()?;
        if let Some(dbqp) = self.using_dbqp.get_mut(&db_names) {
            let mut _rd: f64 = 0.0;
            let mut ctx = AuxContext {
                key2slot: &self.key2slot,
                qp: &mut dbqp.qp,
                onto: &self.onto,
            };
            transform_vql_to_xapian(&mut ctx, &mut tta, None, None, &mut query, &mut _rd, 0)?;
        }

        debug!("query={:?}", query.get_description());

        if query.is_empty() {
            sr.result_code = ResultCode::Ok;
            warn!("query is empty [{}]", request.query);
            return Ok(sr);
        }

        if let Some(dbqp) = self.using_dbqp.get_mut(&db_names) {
            let mut xapian_enquire = dbqp.db.new_enquire()?;

            xapian_enquire.set_query(&mut query)?;

            if let Some(s) = get_sorter(&request.sort, &self.key2slot)? {
                xapian_enquire.set_sort_by_key(s, true)?;
            }

            sr = exec_xapian_query_and_queue_authorize(request, &mut xapian_enquire, add_out_element, op_auth, out_list, &mut self.az).await;
        }

        debug!("res={:?}", sr);
        sr.total_time = total_time.elapsed().as_millis() as i64;
        sr.query_time = sr.total_time - sr.authorize_time;

        Ok(sr)
    }

    pub fn load_index_schema(&mut self, storage: &mut VStorage) {
        fn add_out_element(id: &str, ctx: &mut Vec<String>) {
            ctx.push(id.to_owned());
        }
        let mut ctx = vec![];

        match block_on(self.query_use_collect_fn(
            &FTQuery::new_with_user("cfg:VedaSystem", "'rdf:type' === 'vdi:ClassIndex'"),
            add_out_element,
            OptAuthorize::NO,
            &mut ctx,
        )) {
            Ok(res) => {
                if res.result_code == ResultCode::Ok && res.count > 0 {
                    for id in ctx.iter() {
                        let indv = &mut Individual::default();
                        if storage.get_individual(id, indv) {
                            self.index_schema.add_schema_data(&self.onto, indv);
                        }
                    }
                } else {
                    error!("fail load index schema, err={:?}", res.result_code);
                }
            },
            Err(e) => match e {
                XError::Xapian(code) => {
                    error!("fail load index schema, err={} ({})", get_xapian_err_type(code), code);
                },
                XError::Io(e) => {
                    error!("fail load index schema, err={:?}", e);
                },
            },
        }
    }

    pub async fn c_load_index_schema(&mut self, storage: &AStorage) {
        fn add_out_element(id: &str, ctx: &mut Vec<String>) {
            ctx.push(id.to_owned());
        }
        let mut ctx = vec![];

        info!("start load index schema");
        match self.query_use_collect_fn(&FTQuery::new_with_user("cfg:VedaSystem", "'rdf:type' === 'vdi:ClassIndex'"), add_out_element, OptAuthorize::NO, &mut ctx).await {
            Ok(res) => {
                if res.result_code == ResultCode::Ok && res.count > 0 {
                    for id in ctx.iter() {
                        if let Ok((mut indv, res)) = get_individual_from_db(id, "", storage, None).await {
                            if res == ResultCode::Ok {
                                self.index_schema.add_schema_data(&self.onto, &mut indv);
                            }
                        }
                    }
                } else {
                    error!("fail load index schema, err={:?}", res.result_code);
                }
            },
            Err(e) => match e {
                XError::Xapian(code) => {
                    error!("fail load index schema, err={} ({})", get_xapian_err_type(code), code);
                },
                XError::Io(e) => {
                    error!("fail load index schema, err={:?}", e);
                },
            },
        }

        info!("load index schema, size={}", self.index_schema.len());
    }

    fn reopen_dbs(&mut self) -> Result<()> {
        for (_, el) in self.using_dbqp.iter_mut() {
            el.db.reopen()?;
            el.qp.set_database(&mut el.db)?;
        }

        for (_, db) in self.opened_db.iter_mut() {
            db.reopen()?;
        }

        Ok(())
    }

    fn _close_dbs(&mut self) -> Result<()> {
        for (_, el) in self.using_dbqp.iter_mut() {
            el.db.close()?;
        }

        for (_, db) in self.opened_db.iter_mut() {
            db.close()?;
        }

        Ok(())
    }

    fn open_db_if_need(&mut self, db_name: &str) -> Result<()> {
        if !self.opened_db.contains_key(db_name) {
            if let Some(path) = self.db2path.get(db_name) {
                let db = Database::new_with_path(&("./".to_owned() + path), UNKNOWN)?;
                self.opened_db.insert(db_name.to_owned(), db);
            } else {
                return Err(XError::from(Error::new(ErrorKind::Other, "db2path invalid")));
            }
        }
        Ok(())
    }

    fn open_dbqp_if_need(&mut self, db_names: &[String]) -> Result<()> {
        if !self.using_dbqp.contains_key(db_names) {
            for el in db_names {
                self.open_db_if_need(el)?;
            }

            let mut dbqp = DatabaseQueryParser {
                db: Database::new()?,
                qp: QueryParser::new()?,
            };

            for el in db_names {
                self.open_db_if_need(el)?;
                dbqp.add_database(el, &mut self.opened_db)?;
            }

            dbqp.qp.set_max_wildcard_expansion(MAX_WILDCARD_EXPANSION)?;

            self.xapian_stemmer = Stem::new(&self.xapian_lang)?;

            dbqp.qp.set_stemmer(&mut self.xapian_stemmer)?;

            dbqp.qp.set_database(&mut dbqp.db)?;

            self.using_dbqp.insert(db_names.to_vec(), dbqp);
        }
        /*
           committed_op_id = get_info().committed_op_id;
        */
        Ok(())
    }

    fn get_dn_names(&self, tta: &TTA, db_names_str: &str) -> Vec<String> {
        let mut db_names = vec![];

        if db_names_str.is_empty() {
            let mut databases = HashMap::new();
            self.db_names_from_tta(tta, &mut databases);

            for (key, value) in databases.iter() {
                if !(*value) {
                    if key != "not-indexed" {
                        db_names.push(key.to_owned());
                    }

                    // при автоопределении баз, если находится база deleted, то другие базы исключаются
                    if key == "deleted" {
                        db_names.clear();
                        db_names.push(key.to_owned());
                        break;
                    }
                }
            }
        } else {
            for el in db_names_str.split(',') {
                db_names.push(String::from(el).trim().to_owned());
            }
        }

        if db_names.is_empty() {
            db_names.push("base".to_owned());
        }

        db_names
    }

    fn db_names_from_tta(&self, tta: &TTA, db_names: &mut HashMap<String, bool>) -> String {
        let mut ll = String::default();
        let mut rr = String::default();

        if let Some(l) = &tta.l {
            ll = self.db_names_from_tta(l, db_names)
        };

        if let Some(r) = &tta.r {
            rr = self.db_names_from_tta(r, db_names);
        }

        if !ll.is_empty() && !rr.is_empty() {
            if ll == "rdf:type" {
                let dbn = self.index_schema.get_dbname_of_class(&rr);
                db_names.insert(dbn.to_owned(), false);
            } else if ll == "v-s:deleted" {
                db_names.insert("deleted".to_owned(), false);
            }
        }

        tta.op.to_owned()
    }
}
