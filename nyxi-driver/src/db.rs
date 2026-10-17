use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct DbManager {
    dbs: Arc<Mutex<HashMap<String, sled::Db>>>,
}

impl DbManager {
    pub fn new() -> Self {
        Self {
            dbs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_or_open(&self, dbpath: &str) -> Result<sled::Db, String> {
        let mut dbs = self.dbs.lock();
        if let Some(db) = dbs.get(dbpath) {
            return Ok(db.clone());
        }

        // Ensure parent directory exists
        if let Some(parent) = Path::new(dbpath).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let db = sled::open(dbpath).map_err(|e| format!("Failed to open db {}: {}", dbpath, e))?;
        dbs.insert(dbpath.to_string(), db.clone());
        Ok(db)
    }

    pub fn insert(&self, dbpath: &str, key: &[u8], value: &[u8]) -> Result<(), String> {
        let db = self.get_or_open(dbpath)?;
        db.insert(key, value)
            .map_err(|e| format!("insert error: {}", e))?;
        db.flush().map_err(|e| format!("flush error: {}", e))?;
        Ok(())
    }

    pub fn get(&self, dbpath: &str, key: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let db = self.get_or_open(dbpath)?;
        let val = db
            .get(key)
            .map_err(|e| format!("get error: {}", e))?
            .map(|ivec| ivec.to_vec());
        Ok(val)
    }

    pub fn delete(&self, dbpath: &str, key: &[u8]) -> Result<(), String> {
        let db = self.get_or_open(dbpath)?;
        db.remove(key).map_err(|e| format!("delete error: {}", e))?;
        db.flush().map_err(|e| format!("flush error: {}", e))?;
        Ok(())
    }

    pub fn get_all_keys(&self, dbpath: &str) -> Result<Vec<Vec<u8>>, String> {
        let db = self.get_or_open(dbpath)?;
        let mut keys = Vec::new();
        for item in db.iter().keys() {
            let k = item.map_err(|e| format!("iter key error: {}", e))?;
            keys.push(k.to_vec());
        }
        Ok(keys)
    }

    pub fn get_all_values(&self, dbpath: &str) -> Result<Vec<Vec<u8>>, String> {
        let db = self.get_or_open(dbpath)?;
        let mut values = Vec::new();
        for item in db.iter().values() {
            let v = item.map_err(|e| format!("iter val error: {}", e))?;
            values.push(v.to_vec());
        }
        Ok(values)
    }
}
