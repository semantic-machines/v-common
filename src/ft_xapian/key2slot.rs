use crc32fast::Hasher;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::time::SystemTime;
use xapian_rusty::XError;

pub const XAPIAN_INFO_PATH: &str = "./data/xapian-info";

pub struct Key2Slot {
    data: HashMap<String, u32>,
    last_size_key2slot: usize,
    modified: SystemTime,
}

impl Default for Key2Slot {
    fn default() -> Self {
        Key2Slot {
            data: Default::default(),
            last_size_key2slot: 0,
            modified: SystemTime::now(),
        }
    }
}

impl Key2Slot {
    pub fn new(t: SystemTime) -> Self {
        Key2Slot {
            data: Default::default(),
            last_size_key2slot: 0,
            modified: t,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get_slot(&self, key: &str) -> Option<u32> {
        if key.is_empty() {
            return None;
        }

        if let Some(c) = key.chars().next() {
            if c == '#' {
                if let Ok(v) = key[1..].parse::<u32>() {
                    return Some(v);
                } else {
                    error!("invalid slot: {}", key);
                    return None;
                }
            }
        }

        if let Some(slot) = self.data.get(key) {
            Some(slot.to_owned())
        } else {
            debug!("key2slot, slot not found, key={}", key);
            None
        }
    }

    pub fn get_slot_and_set_if_not_found(&mut self, field: &str) -> u32 {
        if let Some(slot) = self.get_slot(field) {
            return slot;
        }

        // create new slot
        let slot = (self.data.len() + 1) as u32;
        self.data.insert(field.to_owned(), slot);
        if let Err(e) = self.store() {
            error!("fail store key2slot, err={:?}", e);
        } else {
            info!("create new slot {}={}", field, slot);
        }
        slot
    }

    pub fn is_need_reload(&mut self) -> Result<bool, XError> {
        let fname = XAPIAN_INFO_PATH.to_owned() + "/key2slot";
        let cur_modified = fs::metadata(fname)?.modified()?;
        Ok(cur_modified != self.modified)
    }

    pub fn load() -> Result<Key2Slot, XError> {
        let fname = XAPIAN_INFO_PATH.to_owned() + "/key2slot";
        let mut ff = OpenOptions::new().read(true).open(fname)?;
        ff.seek(SeekFrom::Start(0))?;

        let mut key2slot = Key2Slot::new(ff.metadata()?.modified()?);

        let mut hash_in_file = String::default();

        let mut hash = Hasher::new();
        let mut b = BufReader::new(ff);
        let mut rb = vec![];
        b.read_to_end(&mut rb)?;

        let mut start_pos = 0;
        for x in rb.iter() {
            start_pos += 1;
            if x == &0xA {
                break;
            }
        }

        let buff = &rb[start_pos..rb.len()];
        hash.update(buff);

        let new_hash = format!("{:X}", hash.finalize());

        for line in BufReader::new(rb.as_slice()).lines().flatten() {
            let (field, slot) = scan_fmt!(&line, "\"{}\",{}", String, u32);

            if let (Some(f), Some(s)) = (field, slot) {
                if key2slot.is_empty() {
                    hash_in_file = f.to_owned();
                }
                key2slot.data.insert(f, s);
            } else {
                error!("fail parse key2slot, line={}", line);
            }
        }

        if new_hash != hash_in_file {
            error!("key2slot: {} != {}", hash_in_file, new_hash);
            return Err(XError::from(Error::new(ErrorKind::InvalidData, "invalid hash of key2slot".to_string())));
        }

        Ok(key2slot)
    }

    pub fn store(&mut self) -> Result<(), XError> {
        let (data, hash) = self.serialize();

        if data.len() == self.last_size_key2slot {
            return Ok(());
        }

        let mut ff = OpenOptions::new().write(true).truncate(true).create(true).open(XAPIAN_INFO_PATH.to_owned() + "/key2slot")?;
        ff.write_all(format!("\"{}\",{}\n{}", hash, data.len(), data).as_bytes())?;

        Ok(())
    }

    fn serialize(&self) -> (String, String) {
        let mut outbuff = String::new();

        for (key, value) in self.data.iter() {
            outbuff.push('"');
            outbuff.push_str(key);
            outbuff.push('"');
            outbuff.push(',');
            outbuff.push_str(&value.to_string());
            outbuff.push('\n');
        }

        let mut hash = Hasher::new();
        hash.update(outbuff.as_bytes());

        let hash_hex = format!("{:X}", hash.finalize());

        (outbuff, hash_hex)
    }
}
