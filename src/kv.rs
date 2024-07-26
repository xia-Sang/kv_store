use serde_json::Deserializer;

use crate::command::Command;
use crate::error::{KvStoreError, Result};
use std::collections::HashMap;
use std::fs::{create_dir_all, read_dir, remove_file, File, OpenOptions};
use std::io::{self, Read};
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;

const MAX_USELESS_SIZE: u64 = 1024 * 4;
pub struct KvStore {
    index: HashMap<String, Pos>,
    curr_readers: HashMap<u64, BufReader<File>>,
    curr_writer: BufWriterWithPos<File>,
    curr_file_num: u64,
    dir_path: PathBuf,
    useless_size: u64,
}
struct BufWriterWithPos<T: Write + Seek> {
    pos: u64,
    writer: BufWriter<T>,
}
impl<T: Write + Seek> BufWriterWithPos<T> {
    fn new(mut inner: T) -> Result<Self> {
        let pos = inner.seek(SeekFrom::End(0))?;
        Ok(BufWriterWithPos {
            pos,
            writer: BufWriter::new(inner),
        })
    }
    fn get_pos(&self) -> u64 {
        self.pos
    }
}
impl<T: Write + Seek> Write for BufWriterWithPos<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
pub struct Pos {
    file_id: u64,
    offset: u64,
    length: u64,
}

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let dir_path = path.into();
        create_dir_all(&dir_path)?;
        let mut index = HashMap::new();
        let mut curr_readers = HashMap::new();

        let (curr_file_num, useless_size) =
            Self::recover(&dir_path, &mut curr_readers, &mut index)?;

        let curr_file_path = dir_path.join(format!("data_{}.data", curr_file_num));
        let curr_writer = BufWriterWithPos::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&curr_file_path)?,
        )?;
        if curr_file_num == 0 {
            curr_readers.insert(curr_file_num, BufReader::new(File::open(&curr_file_path)?));
        }
        let mut store = KvStore {
            index,
            curr_readers,
            curr_writer,
            curr_file_num,
            dir_path,
            useless_size,
        };
        if store.useless_size > MAX_USELESS_SIZE {
            store.compact()?;
        }
        Ok(store)
    }
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::Set(key, value);
        let data = serde_json::to_vec(&command)?;

        let offset = self.curr_writer.get_pos();
        self.curr_writer.write_all(&data)?;
        self.curr_writer.flush()?;

        let length = self.curr_writer.get_pos() - offset;
        let file_number = self.curr_file_num;

        if let Command::Set(key, _) = command {
            self.useless_size += self
                .index
                .insert(
                    key,
                    Pos {
                        file_id: file_number,
                        offset: offset,
                        length: length,
                    },
                )
                .map(|cp| cp.length)
                .unwrap_or(0);
        }
        Ok(())
    }
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(pos) = self.index.get(&key) {
            let src_reader = self.curr_readers.get_mut(&pos.file_id).expect("msg");
            src_reader.seek(SeekFrom::Start(pos.offset))?;

            let data_reader = src_reader.take(pos.length as u64);
            if let Command::Set(_, value) = serde_json::from_reader(data_reader)? {
                Ok(Some(value))
            } else {
                Err(KvStoreError::UnKnowCommandType)
            }
        } else {
            Ok(None)
        }
    }
    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.index.get(&key).is_some() {
            self.useless_size += self.index.remove(&key).map(|cp| cp.length).unwrap_or(0);

            let command = serde_json::to_vec(&Command::Rm(key))?;
            let offset = self.curr_writer.get_pos();

            self.curr_writer.write_all(&command)?;
            self.curr_writer.flush()?;

            self.useless_size += self.curr_writer.get_pos() - offset;
            Ok(())
        } else {
            Err(KvStoreError::KeyNotFound)
        }
    }
    fn recover(
        dir_path: &PathBuf,
        curr_reader: &mut HashMap<u64, BufReader<File>>,
        index: &mut HashMap<String, Pos>,
    ) -> Result<(u64, u64)> {
        let mut versions: Vec<u64> = read_dir(dir_path)?
            .flat_map(|res| res.map(|e| e.path()))
            .filter(|path| path.is_file())
            .flat_map(|path| {
                path.file_name()
                    .and_then(|filename| filename.to_str())
                    .map(|filename| {
                        filename
                            .trim_start_matches("data_")
                            .trim_end_matches(".data")
                    })
                    .map(str::parse::<u64>)
            })
            .flatten()
            .collect();
        versions.sort();

        let mut useless_size = 0;
        for version in &versions {
            let file_path = dir_path.join(format!("data_{}.data", version));
            let reader = BufReader::new(File::open(&file_path)?);
            let mut iter = Deserializer::from_reader(reader).into_iter::<Command>();
            let mut before_offset = iter.byte_offset() as u64;
            while let Some(command) = iter.next() {
                let after_offset = iter.byte_offset() as u64;
                let length = after_offset - before_offset;
                match command? {
                    Command::Set(key, _) => {
                        useless_size += index
                            .insert(
                                key,
                                Pos {
                                    file_id: *version,
                                    offset: before_offset,
                                    length: length,
                                },
                            )
                            .map(|cp| cp.length)
                            .unwrap_or(0);
                    }
                    Command::Rm(key) => {
                        useless_size += index.remove(&key).map(|cp| cp.length).unwrap_or(0);
                        useless_size += length;
                    }
                };
                before_offset = after_offset;
            }
            curr_reader.insert(*version, BufReader::new(File::open(&file_path)?));
        }
        Ok((*versions.last().unwrap_or(&0), useless_size))
    }
    fn create_new_file(&mut self) -> Result<()> {
        self.curr_file_num += 1;
        let new_file_path = self
            .dir_path
            .join(format!("data_{}.data", self.curr_file_num));
        self.curr_writer = BufWriterWithPos::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&new_file_path)?,
        )?;

        self.curr_readers.insert(
            self.curr_file_num,
            BufReader::new(File::open(new_file_path)?),
        );
        Ok(())
    }

    fn compact(&mut self) -> Result<()> {
        self.create_new_file()?;
        let mut before_offset = 0;

        for pos in self.index.values_mut() {
            let src_reader = self.curr_readers.get_mut(&pos.file_id).expect("msg");

            src_reader.seek(SeekFrom::Start(pos.offset))?;

            let mut data_reader = src_reader.take(pos.length);
            io::copy(&mut data_reader, &mut self.curr_writer)?;
            let after_offset = self.curr_writer.pos;
            *pos = Pos {
                offset: before_offset,
                length: after_offset - before_offset,
                file_id: self.curr_file_num,
            };
            before_offset = after_offset;
        }
        self.curr_writer.flush()?;

        // let delete_file_num: Vec<u64> = self
        //     .curr_readers
        //     .iter()
        //     .map(|(key, _)| *key)
        //     .filter(|key| *key < self.curr_file_num)
        //     .collect();
        let delete_file_num: Vec<u64> = self
            .curr_readers
            .keys()
            .map(|key| *key)
            .filter(|key| *key < self.curr_file_num)
            .collect();
        for num in delete_file_num {
            self.curr_readers.remove(&num);
            remove_file(self.dir_path.join(format!("data_{}.data", num)))?;
        }
        self.create_new_file()?;
        Ok(())
    }
}
