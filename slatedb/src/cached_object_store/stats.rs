use slatedb_common::metrics::{CounterFn, GaugeFn, HistogramFn, MetricsRecorderHelper};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

macro_rules! oscache_stat_name {
    ($suffix:expr) => {
        concat!("slatedb.object_store_cache.", $suffix)
    };
}

pub const PART_HIT_COUNT: &str = oscache_stat_name!("part_hit_count");
pub const PART_ACCESS_COUNT: &str = oscache_stat_name!("part_access_count");
pub const CACHE_KEYS: &str = oscache_stat_name!("cache_keys");
pub const CACHE_BYTES: &str = oscache_stat_name!("cache_bytes");
pub const EVICTED_KEYS: &str = oscache_stat_name!("evicted_keys");
pub const EVICTED_BYTES: &str = oscache_stat_name!("evicted_bytes");

/// Wall-clock duration of the `pread` that serves a cached data part.
pub const PART_READ_DURATION: &str = oscache_stat_name!("part_read_duration");
/// Wall-clock duration of the `fstat` + `pread` that reads a cache head file.
pub const HEAD_READ_DURATION: &str = oscache_stat_name!("head_read_duration");
/// Wall-clock duration of `open(2)` on a file-handle-cache miss.
pub const FILE_OPEN_DURATION: &str = oscache_stat_name!("file_open_duration");
/// Wall-clock duration of the `write(2)` calls that fill the temp file.
pub const WRITE_DURATION: &str = oscache_stat_name!("write_duration");
/// Wall-clock duration of `fsync(2)` alone, excluding every other write step.
pub const FSYNC_DURATION: &str = oscache_stat_name!("fsync_duration");
/// Wall-clock duration of the namespace operations around a cache write:
/// `create_dir_all` + `open(tmp)` + `rename(tmp, final)`.
pub const METADATA_DURATION: &str = oscache_stat_name!("metadata_duration");

/// Histogram boundaries, in seconds, for disk-cache I/O latency.
///
/// [`slatedb_common::metrics::LATENCY_BOUNDARIES`] bottoms out at 1ms, which is
/// already above the median for a local NVMe/gp3 `pread` and would collapse the
/// whole local-disk distribution into the first bucket. These boundaries start
/// at 25us so that a local-disk baseline is actually resolvable, while still
/// running out to 2.5s so that a network filesystem (NFS/EFS), where an `fsync`
/// is a round-trip commit rather than a page-cache flush, does not saturate the
/// overflow bucket.
pub const IO_LATENCY_BOUNDARIES: &[f64] = &[
    0.000_025, 0.000_05, 0.000_1, 0.000_25, 0.000_5, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1,
    0.25, 0.5, 1.0, 2.5,
];

#[derive(Clone)]
pub struct CachedObjectStoreStats {
    pub(super) object_store_cache_part_hits: Arc<dyn CounterFn>,
    pub(super) object_store_cache_part_access: Arc<dyn CounterFn>,
    pub(super) object_store_cache_keys: Arc<dyn GaugeFn>,
    pub(super) object_store_cache_bytes: Arc<dyn GaugeFn>,
    pub(super) object_store_cache_evicted_keys: Arc<dyn CounterFn>,
    pub(super) object_store_cache_evicted_bytes: Arc<dyn CounterFn>,
    pub(super) object_store_cache_part_read_duration: Arc<dyn HistogramFn>,
    pub(super) object_store_cache_head_read_duration: Arc<dyn HistogramFn>,
    pub(super) object_store_cache_file_open_duration: Arc<dyn HistogramFn>,
    pub(super) object_store_cache_write_duration: Arc<dyn HistogramFn>,
    pub(super) object_store_cache_fsync_duration: Arc<dyn HistogramFn>,
    pub(super) object_store_cache_metadata_duration: Arc<dyn HistogramFn>,
}

impl Debug for CachedObjectStoreStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedObjectStoreStats")
            .field("object_store_cache_part_hits", &"<counter>")
            .field("object_store_cache_part_access", &"<counter>")
            .field("object_store_cache_keys", &"<gauge>")
            .field("object_store_cache_bytes", &"<gauge>")
            .field("object_store_cache_evicted_keys", &"<counter>")
            .field("object_store_cache_evicted_bytes", &"<counter>")
            .field("object_store_cache_part_read_duration", &"<histogram>")
            .field("object_store_cache_head_read_duration", &"<histogram>")
            .field("object_store_cache_file_open_duration", &"<histogram>")
            .field("object_store_cache_write_duration", &"<histogram>")
            .field("object_store_cache_fsync_duration", &"<histogram>")
            .field("object_store_cache_metadata_duration", &"<histogram>")
            .finish()
    }
}

impl CachedObjectStoreStats {
    pub(crate) fn new(recorder: &MetricsRecorderHelper) -> Self {
        let latency = |name: &str, description: &str| {
            recorder
                .histogram(name, IO_LATENCY_BOUNDARIES)
                .description(description)
                .register()
        };
        Self {
            object_store_cache_part_hits: recorder.counter(PART_HIT_COUNT).register(),
            object_store_cache_part_access: recorder.counter(PART_ACCESS_COUNT).register(),
            object_store_cache_keys: recorder.gauge(CACHE_KEYS).register(),
            object_store_cache_bytes: recorder.gauge(CACHE_BYTES).register(),
            object_store_cache_evicted_keys: recorder.counter(EVICTED_KEYS).register(),
            object_store_cache_evicted_bytes: recorder.counter(EVICTED_BYTES).register(),
            object_store_cache_part_read_duration: latency(
                PART_READ_DURATION,
                "Seconds spent in the positional read that serves a cached data part",
            ),
            object_store_cache_head_read_duration: latency(
                HEAD_READ_DURATION,
                "Seconds spent reading a cache head (metadata) file from disk",
            ),
            object_store_cache_file_open_duration: latency(
                FILE_OPEN_DURATION,
                "Seconds spent in open(2) when the file handle cache misses",
            ),
            object_store_cache_write_duration: latency(
                WRITE_DURATION,
                "Seconds spent writing cache bytes to the temp file, excluding fsync",
            ),
            object_store_cache_fsync_duration: latency(
                FSYNC_DURATION,
                "Seconds spent in fsync(2) on a cache write, measured on its own",
            ),
            object_store_cache_metadata_duration: latency(
                METADATA_DURATION,
                "Seconds spent in create_dir_all + open + rename on a cache write",
            ),
        }
    }
}
