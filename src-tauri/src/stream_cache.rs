//! Shared streaming engine for the remote video protocols (`gdrive://`, `srv://`).
//!
//! A `<video>` element parses media with hundreds of small range requests, and a
//! network round-trip per request stalls playback outright. Both remote protocols
//! therefore serve from this block cache, which provides what smooth playback on
//! a slow connection actually needs:
//!
//! * **Aligned 2 MB blocks in a bounded FIFO cache** (~64 MB) — the player's tiny
//!   reads collapse into a handful of network fetches, and scrubbing through a
//!   long video can't grow memory without bound.
//! * **In-flight de-duplication** — concurrent requests that touch the same
//!   missing block share one network fetch instead of racing duplicates.
//! * **Background read-ahead** — after every response the next few blocks are
//!   fetched sequentially in the background, so steady playback almost always
//!   hits the cache instead of pausing a round-trip every couple of megabytes.
//! * **Fast first byte** — an open-ended request (`bytes=N-`) is answered as soon
//!   as its first block is available, plus whatever contiguous blocks are already
//!   cached. The critical path never waits on more than one network fetch.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::http::{header, Method, Request, Response, StatusCode};

/// Cache/fetch granularity.
const BLOCK: u64 = 2 * 1024 * 1024;
/// Ceiling on cached blocks for the current file (~64 MB).
const MAX_BLOCKS: usize = 32;
/// How many blocks past the last response to warm in the background (~16 MB,
/// i.e. tens of seconds of typical course video).
const READAHEAD_BLOCKS: u64 = 8;
/// Ceiling on one open-ended (`bytes=N-`) response.
const OPEN_ENDED_CAP: u64 = 8 * 1024 * 1024;

pub type FetchFut = Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send>>;
/// Fetches an inclusive byte range `[start, end]` from the source.
pub type FetchFn = Arc<dyn Fn(u64, u64) -> FetchFut + Send + Sync>;

/// Distinguishes cache resets so stale read-ahead chains and in-flight fetches
/// die quietly when a different file starts streaming.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Cache for the single file currently streaming — the player plays one video at
/// a time, so a new file resets it.
struct FileState {
    key: String,
    generation: u64,
    total: u64,
    content_type: String,
    fetch: FetchFn,
    blocks: HashMap<u64, Arc<Vec<u8>>>,
    /// Insertion order, for evicting the oldest block past `MAX_BLOCKS`.
    order: VecDeque<u64>,
    /// Per-block fetch locks: waiters queue here instead of duplicating a fetch.
    locks: HashMap<u64, Arc<tokio::sync::Mutex<()>>>,
    /// Last block a response ended in — read-ahead chains follow it, and a chain
    /// stops as soon as a newer response supersedes it.
    last_block: u64,
}

pub struct Engine {
    label: &'static str,
    state: Mutex<Option<FileState>>,
}

impl Engine {
    pub const fn new(label: &'static str) -> Self {
        Engine {
            label,
            state: Mutex::new(None),
        }
    }

    /// Serve one protocol request for `key`. `meta` is awaited only when `key`
    /// isn't the file already cached; it resolves to (total size, content type).
    pub async fn serve(
        &'static self,
        key: &str,
        request: &Request<Vec<u8>>,
        meta: impl Future<Output = Result<(u64, String), String>>,
        fetch: FetchFn,
    ) -> Response<Vec<u8>> {
        let (total, content_type, generation) = match self.ensure_meta(key, meta, fetch).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[{}] meta error for {key}: {e}", self.label);
                return status_only(StatusCode::BAD_GATEWAY);
            }
        };

        if request.method() == Method::HEAD {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CONTENT_LENGTH, total.to_string())
                .header(header::ACCEPT_RANGES, "bytes")
                .body(Vec::new())
                .unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR));
        }

        if total == 0 {
            return status_only(StatusCode::INTERNAL_SERVER_ERROR);
        }

        let (start, end_opt) = parse_req_range(request);
        if start >= total {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .body(Vec::new())
                .unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR));
        }
        let open_ended = end_opt.is_none();
        let end = end_opt.unwrap_or(start + OPEN_ENDED_CAP - 1).min(total - 1);

        // Assemble [start..=end]. An explicit-end request is satisfied in full
        // (the media stack uses those for small probes and expects every byte);
        // an open-ended one waits for at most its first block and then takes
        // only what's already cached, so slow networks still get bytes flowing
        // after a single fetch.
        let mut body: Vec<u8> = Vec::with_capacity((end - start + 1) as usize);
        let mut pos = start;
        while pos <= end {
            let block_idx = pos / BLOCK;
            let block = if open_ended && !body.is_empty() {
                match self.peek(generation, block_idx) {
                    Some(b) => b,
                    None => break,
                }
            } else {
                match self.get_block(generation, block_idx).await {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("[{}] block {block_idx} fetch error: {e}", self.label);
                        return status_only(StatusCode::BAD_GATEWAY);
                    }
                }
            };
            let block_start = block_idx * BLOCK;
            let block_end_global = block_start + block.len() as u64; // exclusive
            if pos >= block_end_global {
                break; // safety: short final block
            }
            let off = (pos - block_start) as usize;
            let copy_end = (end + 1).min(block_end_global); // exclusive
            let len = (copy_end - pos) as usize;
            body.extend_from_slice(&block[off..off + len]);
            pos = copy_end;
        }

        if body.is_empty() {
            return status_only(StatusCode::INTERNAL_SERVER_ERROR);
        }
        let actual_end = start + body.len() as u64 - 1;

        self.spawn_readahead(generation, actual_end / BLOCK);

        Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_LENGTH, body.len().to_string())
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{actual_end}/{total}"),
            )
            .body(body)
            .unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR))
    }

    /// Return the cached state for `key`, initialising it (and dropping any
    /// previous file's cache) via `meta` when a different file starts.
    async fn ensure_meta(
        &self,
        key: &str,
        meta: impl Future<Output = Result<(u64, String), String>>,
        fetch: FetchFn,
    ) -> Result<(u64, String, u64), String> {
        {
            let guard = self.state.lock().map_err(|e| e.to_string())?;
            if let Some(fs) = guard.as_ref() {
                if fs.key == key {
                    return Ok((fs.total, fs.content_type.clone(), fs.generation));
                }
            }
        }

        let (total, content_type) = meta.await?;
        eprintln!(
            "[{}] meta {key} total={total} type={content_type}",
            self.label
        );

        let mut guard = self.state.lock().map_err(|e| e.to_string())?;
        // A concurrent request may have initialised the same file meanwhile.
        if let Some(fs) = guard.as_ref() {
            if fs.key == key {
                return Ok((fs.total, fs.content_type.clone(), fs.generation));
            }
        }
        let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        *guard = Some(FileState {
            key: key.to_string(),
            generation,
            total,
            content_type: content_type.clone(),
            fetch,
            blocks: HashMap::new(),
            order: VecDeque::new(),
            locks: HashMap::new(),
            last_block: 0,
        });
        Ok((total, content_type, generation))
    }

    /// A cached block, or `None` — never touches the network.
    fn peek(&self, generation: u64, idx: u64) -> Option<Arc<Vec<u8>>> {
        let guard = self.state.lock().ok()?;
        let fs = guard.as_ref()?;
        if fs.generation != generation {
            return None;
        }
        fs.blocks.get(&idx).cloned()
    }

    /// A cached block, fetching it on a miss. Concurrent callers for the same
    /// block queue on a per-block lock and share the one fetch's result.
    async fn get_block(&self, generation: u64, idx: u64) -> Result<Arc<Vec<u8>>, String> {
        let (lock, fetch, total) = {
            let mut guard = self.state.lock().map_err(|e| e.to_string())?;
            let fs = match guard.as_mut() {
                Some(fs) if fs.generation == generation => fs,
                _ => return Err("superseded by a newer file".to_string()),
            };
            if let Some(b) = fs.blocks.get(&idx) {
                return Ok(b.clone());
            }
            let lock = fs
                .locks
                .entry(idx)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone();
            (lock, fs.fetch.clone(), fs.total)
        };

        let _fetching = lock.lock().await;

        // Whoever held the lock before us may have already cached the block.
        if let Some(b) = self.peek(generation, idx) {
            return Ok(b);
        }

        let block_start = idx * BLOCK;
        let block_end = ((idx + 1) * BLOCK).min(total) - 1; // inclusive
        let fetched = fetch(block_start, block_end).await;

        let mut guard = self.state.lock().map_err(|e| e.to_string())?;
        let fs = match guard.as_mut() {
            Some(fs) if fs.generation == generation => fs,
            _ => return Err("superseded by a newer file".to_string()),
        };
        fs.locks.remove(&idx);
        let arc = Arc::new(fetched?);
        if fs.blocks.insert(idx, arc.clone()).is_none() {
            fs.order.push_back(idx);
            while fs.order.len() > MAX_BLOCKS {
                if let Some(oldest) = fs.order.pop_front() {
                    fs.blocks.remove(&oldest);
                }
            }
        }
        Ok(arc)
    }

    /// Warm the blocks after `base` in the background, one at a time. Each new
    /// response starts its own chain and retires older ones, so the read-ahead
    /// follows playback (and a seek abandons the old position immediately).
    fn spawn_readahead(&'static self, generation: u64, base: u64) {
        {
            let Ok(mut guard) = self.state.lock() else {
                return;
            };
            match guard.as_mut() {
                Some(fs) if fs.generation == generation => fs.last_block = base,
                _ => return,
            }
        }

        tauri::async_runtime::spawn(async move {
            for step in 1..=READAHEAD_BLOCKS {
                let idx = base + step;
                let still_current = {
                    let Ok(guard) = self.state.lock() else { return };
                    match guard.as_ref() {
                        Some(fs) if fs.generation == generation && fs.last_block == base => {
                            idx * BLOCK < fs.total
                        }
                        _ => false,
                    }
                };
                if !still_current {
                    return;
                }
                if self.get_block(generation, idx).await.is_err() {
                    return;
                }
            }
        });
    }
}

/// Parse the request's Range header into (start, optional end). Defaults to (0, None).
fn parse_req_range(request: &Request<Vec<u8>>) -> (u64, Option<u64>) {
    let raw = match request
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => return (0, None),
    };
    let spec = match raw.strip_prefix("bytes=") {
        Some(s) => s,
        None => return (0, None),
    };
    let (a, b) = match spec.split_once('-') {
        Some(parts) => parts,
        None => return (0, None),
    };
    let start: u64 = a.trim().parse().unwrap_or(0);
    let end = b.trim();
    let end_opt = if end.is_empty() {
        None
    } else {
        end.parse::<u64>().ok()
    };
    (start, end_opt)
}

pub fn status_only(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .body(Vec::new())
        .expect("status-only response")
}
