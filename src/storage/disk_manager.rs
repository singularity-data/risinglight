use super::*;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
pub static DEFAULT_STROAGE_FILE_NAME: &str = "risinglight.db";

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub enum InitMode {
    Create,
    Open,
}
// DiskManager is responsible for managing blocks on disk.
// So we don't need use Mutex.
pub struct DiskManager {
    // We hope user don't have a huge SSD, so the id will not overflow lol. :)
    inner: Mutex<DiskManagerInner>,
}

pub struct DiskManagerInner {
    next_block_id: BlockId,
    file: Option<File>,
}

// Metablock always starts with Block 0. There will be more than one Metablock.
// TODO: Support erasing blocks.
impl DiskManagerInner {
    fn new() -> Self {
        DiskManagerInner {
            next_block_id: 0,
            file: None,
        }
    }
    // Read and Write block will be used by DiskManager in other functions.
    // So we add methods for DiskManagerInner, so DiskManager does not need to grab mutex for twice.
    pub fn read_meta_block(&mut self) {
        self.file
            .as_ref()
            .unwrap()
            .seek(SeekFrom::Start(0))
            .unwrap();
        let mut bytes: [u8; 4] = [0; 4];
        self.file.as_ref().unwrap().read_exact(&mut bytes).unwrap();
        self.next_block_id = u32::from_le_bytes(bytes);
    }

    pub fn write_meta_block(&mut self) {
        self.file
            .as_ref()
            .unwrap()
            .seek(SeekFrom::Start(0))
            .unwrap();
        self.file
            .as_ref()
            .unwrap()
            .write_all(&self.next_block_id.to_le_bytes())
            .unwrap();
    }
}

impl Default for DiskManager {
    fn default() -> Self {
        Self::new()
    }
}
// We won't use Result in DiskManager, the system cannot run anymore and must crash when there is IO error.
impl DiskManager {
    pub fn new() -> DiskManager {
        DiskManager {
            inner: Mutex::new(DiskManagerInner::new()),
        }
    }

    pub fn get_next_block_id(&mut self) -> BlockId {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_block_id;
        inner.next_block_id += 1;
        inner.write_meta_block();
        id
    }
    // The init mode should be decided by OnDisk Storage Manager.
    pub fn init(&mut self, mode: InitMode) {
        match mode {
            // Create mode will create and truncate file.
            InitMode::Create => {
                let temp_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(DEFAULT_STROAGE_FILE_NAME)
                    .unwrap();
                let mut inner = self.inner.lock().unwrap();
                inner.file = Some(temp_file);
                inner.next_block_id = 1;
                inner.write_meta_block();
            }
            // Open mode will open an existing db file, it will be PANIC if failed!
            InitMode::Open => {
                let temp_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(DEFAULT_STROAGE_FILE_NAME)
                    .unwrap();
                let mut inner = self.inner.lock().unwrap();
                inner.file = Some(temp_file);
                inner.read_meta_block();
            }
        }
    }

    pub fn write_block(&mut self, block_id: BlockId, block: Arc<Block>) {
        let inner = self.inner.lock().unwrap();
        inner
            .file
            .as_ref()
            .unwrap()
            .seek(SeekFrom::Start(block_id as u64 * BLOCK_SIZE as u64))
            .unwrap();
        inner
            .file
            .as_ref()
            .unwrap()
            .write_all(block.get_inner_mutex().get_buffer_ref())
            .unwrap();
    }

    pub fn read_block(&mut self, block_id: BlockId) -> Arc<Block> {
        let block = Block::new();
        let inner = self.inner.lock().unwrap();
        inner
            .file
            .as_ref()
            .unwrap()
            .seek(SeekFrom::Start(block_id as u64 * BLOCK_SIZE as u64))
            .unwrap();
        inner
            .file
            .as_ref()
            .unwrap()
            .read_exact(block.get_inner_mutex().get_buffer_mut())
            .unwrap();
        Arc::new(block)
    }
}
