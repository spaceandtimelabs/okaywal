use std::{io, ops::Mul, path::Path, sync::Arc};

use file_manager::{fs::StdFileManager, FileManager, PathId};

use crate::{LogManager, WriteAheadLog};

/// A [`WriteAheadLog`] configuration.
#[derive(Debug, Clone)]
#[must_use]
pub struct Configuration<M> {
    /// The file manager to use for storing data.
    ///
    /// Typically this is [`StdFileManager`].
    pub file_manager: M,
    /// The directory to store the log files in.
    pub directory: PathId,
    /// The number of bytes each log file should be preallocated with. Log files
    /// may grow to be larger than this size if needed.
    pub preallocate_bytes: u32,
    /// After this many bytes have been written to the active log file, begin a
    /// checkpointing process. This number should be less than
    /// `preallocate_bytes` to try to ensure the checkpointing process happens
    /// before the preallocated space is fully exhausted. If this amount is too
    /// close to the preallocation amount, an entry being written may need to
    /// extend the file which is a slow operation.
    pub checkpoint_after_bytes: u64,
    /// The number of bytes to use for the in-memory buffer when reading and
    /// writing from the log.
    pub buffer_bytes: usize,
    /// An arbitrary chunk of bytes that is stored in the log files. Limited to
    /// 255 bytes. This can be used for any purpose, but the design inspiration
    /// was to allow detection of what format or version of a format the data
    /// was inside of the log without needing to parse the entries.
    pub version_info: Arc<Vec<u8>>,
    /// The maximum disk usage, in percent, before writes start to be rejected.
    /// Must be a value between 0 and 100.
    pub max_disk_usage_percent: u16,
}

impl Default for Configuration<StdFileManager> {
    fn default() -> Self {
        Self::default_for("okaywal")
    }
}

impl Configuration<StdFileManager> {
    /// Returns the default configuration for a given directory.
    ///
    /// This currently is:
    ///
    /// - `preallocate_bytes`: 1 megabyte
    /// - `checkpoint_after_bytes`: 768 kilobytes
    /// - `buffer_bytes`: 16 kilobytes
    pub fn default_for<P: AsRef<Path>>(path: P) -> Self {
        Self::default_with_manager(path, StdFileManager::default())
    }
}

impl<M> Configuration<M>
where
    M: FileManager,
{
    /// Returns the default configuration for a given directory and file manager.
    ///
    /// This currently is:
    ///
    /// - `preallocate_bytes`: 1 megabyte
    /// - `checkpoint_after_bytes`: 768 kilobytes
    /// - `buffer_bytes`: 16 kilobytes
    pub fn default_with_manager<P: AsRef<Path>>(path: P, file_manager: M) -> Self {
        Self {
            file_manager,
            directory: PathId::from(path.as_ref()),
            preallocate_bytes: megabytes(1),
            checkpoint_after_bytes: kilobytes(768),
            buffer_bytes: kilobytes(16),
            version_info: Arc::default(),
            max_disk_usage_percent: 95,
        }
    }
    /// Sets the number of bytes to preallocate for each segment file. Returns `self`.
    ///
    /// Preallocating disk space allows for more consistent performance. This
    /// number should be large enough to allow batching multiple entries into
    /// one checkpointing operation.
    pub fn preallocate_bytes(mut self, bytes: u32) -> Self {
        self.preallocate_bytes = bytes;
        self
    }

    /// Sets the number of bytes written required to begin a checkpoint
    /// operation. Returns `self`.
    ///
    /// This value should be smaller than `preallocate_bytes` to ensure
    /// checkpoint operations begin before too much data is written in a log
    /// entry. If more data is written before a checkpoint occurs, the segment
    /// will grow to accomodate the extra data, but that write will not be as
    /// fast due to needing to allocate more space from the filesystem to
    /// perform the write.
    pub fn checkpoint_after_bytes(mut self, bytes: u64) -> Self {
        self.checkpoint_after_bytes = bytes;
        self
    }

    /// Sets the number of bytes to use for internal buffers when reading and
    /// writing data to the log. Returns `self`.
    pub fn buffer_bytes(mut self, bytes: usize) -> Self {
        self.buffer_bytes = bytes;
        self
    }

    /// Opens the log using the provided log manager with this configuration.
    pub fn open<Manager: LogManager<M>>(self, manager: Manager) -> io::Result<WriteAheadLog<M>> {
        WriteAheadLog::open(self, manager)
    }
}

fn megabytes<T: Mul<Output = T> + From<u16>>(megs: T) -> T {
    kilobytes(megs) * T::from(1024)
}

fn kilobytes<T: Mul<Output = T> + From<u16>>(bytes: T) -> T {
    bytes * T::from(1024)
}
