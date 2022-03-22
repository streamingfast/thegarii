// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![allow(unused)]
use crate::{env, types::FirehoseBlock, Error, Result};
use rocksdb::{IteratorMode, WriteBatch, DB};
use std::path::{Path, PathBuf};

/// firehose block storage
pub struct Storage(pub DB);

impl Storage {
    /// new storage
    pub fn new(db_path: &dyn AsRef<Path>) -> Result<Self> {
        Ok(Self(DB::open_default(db_path)?))
    }

    /// map storage keys
    pub fn map_keys<T>(&self, map: fn(&[u8], &[u8]) -> T) -> Vec<T> {
        self.0
            .iterator(IteratorMode::Start)
            .map(|(k, v)| map(&k, &v))
            .collect::<Vec<T>>()
    }

    /// get missing block numbers
    pub fn missing<Blocks>(&self, keys: Blocks) -> Vec<u64>
    where
        Blocks: Iterator<Item = u64> + Sized,
    {
        keys.into_iter()
            .filter(|key| !self.0.key_may_exist(key.to_le_bytes()))
            .collect()
    }

    /// count blocks
    ///
    /// see https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
    pub fn count(&self) -> Result<u64> {
        Ok(self
            .0
            .property_int_value("rocksdb.estimate-num-keys")?
            .unwrap_or(0))
    }

    /// get the last block
    pub fn last(&self) -> Result<FirehoseBlock> {
        let (_, value) = self
            .0
            .iterator(IteratorMode::End)
            .next()
            .ok_or(Error::NoBlockExists)?;

        Ok(bincode::deserialize(&value)?)
    }

    /// get block
    pub fn get(&self, height: u64) -> Result<FirehoseBlock> {
        let block_bytes = self
            .0
            .get(height.to_le_bytes())?
            .ok_or(Error::BlockNotFound(height))?;

        Ok(bincode::deserialize(&block_bytes)?)
    }

    /// set block
    pub fn put(&self, block: FirehoseBlock) -> Result<()> {
        self.0
            .put(block.height.to_le_bytes(), &bincode::serialize(&block)?)?;

        Ok(())
    }

    /// new read-only storage
    pub fn read_only(db_path: &dyn AsRef<Path>) -> Result<Self> {
        Ok(Self(DB::open_for_read_only(
            &Default::default(),
            db_path,
            false,
        )?))
    }

    /// batch write blocks into db
    pub fn write(&self, blocks: Vec<FirehoseBlock>) -> Result<()> {
        let mut batch = WriteBatch::default();
        for b in blocks {
            batch.put(b.height.to_le_bytes(), bincode::serialize(&b)?);
        }

        self.0.write(batch)?;
        Ok(())
    }

    /// flush data to disk
    pub fn flush(&self) -> Result<()> {
        self.0.flush()?;

        Ok(())
    }
}