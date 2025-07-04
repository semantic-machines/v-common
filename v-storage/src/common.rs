use v_individual_model::onto::individual::Individual;

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum StorageMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum StorageId {
    Individuals,
    Tickets,
    Az,
}

/// Унифицированный результат операций с хранилищем
#[derive(Debug, Clone, PartialEq)]
pub enum StorageResult<T> {
    Ok(T),
    NotFound,
    NotReady,
    UnprocessableEntity,
    Error(String),
}

impl<T> StorageResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, StorageResult::Ok(_))
    }

    pub fn is_error(&self) -> bool {
        !self.is_ok()
    }

    pub fn unwrap_or_default(self) -> T 
    where 
        T: Default,
    {
        match self {
            StorageResult::Ok(value) => value,
            _ => T::default(),
        }
    }

    pub fn map<U, F>(self, f: F) -> StorageResult<U>
    where 
        F: FnOnce(T) -> U,
    {
        match self {
            StorageResult::Ok(value) => StorageResult::Ok(f(value)),
            StorageResult::NotFound => StorageResult::NotFound,
            StorageResult::NotReady => StorageResult::NotReady,
            StorageResult::UnprocessableEntity => StorageResult::UnprocessableEntity,
            StorageResult::Error(msg) => StorageResult::Error(msg),
        }
    }

    pub fn and_then<U, F>(self, f: F) -> StorageResult<U>
    where 
        F: FnOnce(T) -> StorageResult<U>,
    {
        match self {
            StorageResult::Ok(value) => f(value),
            StorageResult::NotFound => StorageResult::NotFound,
            StorageResult::NotReady => StorageResult::NotReady,
            StorageResult::UnprocessableEntity => StorageResult::UnprocessableEntity,
            StorageResult::Error(msg) => StorageResult::Error(msg),
        }
    }
}

impl<T> From<StorageResult<T>> for bool {
    fn from(result: StorageResult<T>) -> Self {
        result.is_ok()
    }
}

pub trait Storage {
    fn get_individual(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()>;
    fn get_value(&mut self, storage: StorageId, key: &str) -> StorageResult<String>;
    fn get_raw_value(&mut self, storage: StorageId, key: &str) -> StorageResult<Vec<u8>>;
    fn put_value(&mut self, storage: StorageId, key: &str, val: &str) -> StorageResult<()>;
    fn put_raw_value(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> StorageResult<()>;
    fn remove_value(&mut self, storage: StorageId, key: &str) -> StorageResult<()>;
    fn count(&mut self, storage: StorageId) -> StorageResult<usize>;

    // Deprecated methods for backward compatibility
    #[deprecated(since = "0.1.0", note = "Use get_individual instead")]
    fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.get_individual(storage, id, iraw)
    }

    #[deprecated(since = "0.1.0", note = "Use get_value instead")]
    fn get_v(&mut self, storage: StorageId, key: &str) -> Option<String> {
        match self.get_value(storage, key) {
            StorageResult::Ok(value) => Some(value),
            _ => None,
        }
    }

    #[deprecated(since = "0.1.0", note = "Use get_raw_value instead")]
    fn get_raw(&mut self, storage: StorageId, key: &str) -> Vec<u8> {
        self.get_raw_value(storage, key).unwrap_or_default()
    }

    #[deprecated(since = "0.1.0", note = "Use put_value instead")]
    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool {
        self.put_value(storage, key, val).is_ok()
    }

    #[deprecated(since = "0.1.0", note = "Use put_raw_value instead")]
    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool {
        self.put_raw_value(storage, key, val).is_ok()
    }

    #[deprecated(since = "0.1.0", note = "Use remove_value instead")]
    fn remove(&mut self, storage: StorageId, key: &str) -> bool {
        self.remove_value(storage, key).is_ok()
    }
}

/// Макрос для устранения дублирования кода диспетчеризации
#[macro_export]
macro_rules! impl_storage_dispatcher {
    ($self:ident, $storage_field:expr, $operation:expr, $default:expr) => {
        match $storage_field {
            Some(storage) => $operation(storage),
            None => $default,
        }
    };
}

/// Хелпер-трейт для упрощения диспетчеризации
pub trait StorageDispatcher {
    type Storage;

    fn with_storage<T, F>(&mut self, default_value: T, operation: F) -> T
    where
        F: FnOnce(&mut Self::Storage) -> T;

    fn with_storage_result<F>(&mut self, operation: F) -> StorageResult<()>
    where
        F: FnOnce(&mut Self::Storage) -> StorageResult<()>,
    {
        self.with_storage(StorageResult::NotReady, operation)
    }

    fn with_storage_value<T, F>(&mut self, operation: F) -> StorageResult<T>
    where
        F: FnOnce(&mut Self::Storage) -> StorageResult<T>,
        T: Default,
    {
        self.with_storage(StorageResult::NotReady, operation)
    }
}
