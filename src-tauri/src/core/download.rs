//! Resumable, verified downloader (plan P4.2).
//!
//! Each file streams into `<file>.part`. An existing part resumes with a
//! `Range` request: `206` appends, `200` means the server ignored the
//! range so the part restarts from zero, `416` means the part may already
//! be complete and is verified on disk. Bytes are hashed while written;
//! on completion size and SHA-256 are checked and the part is renamed.
//! Cancelling keeps the part for the next attempt.
//!
//! A download never hangs: connecting and every wait for data are timed,
//! a stalled connection is dropped, and a dropped connection is retried
//! from where the part file ends. Cancel is noticed within a quarter of a
//! second even when no data is arriving.

use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How long connecting, or waiting for the reply headers, may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// A connection that delivers nothing for this long is dropped.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
/// How often a wait checks the cancel flag.
const CANCEL_POLL: Duration = Duration::from_millis(250);
/// Extra attempts after a connection problem, each resuming the part.
const NETWORK_RETRIES: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSpec {
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub dest: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Progress {
    pub file_index: usize,
    pub files: usize,
    /// Bytes on disk across all files, including resumed parts.
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadError {
    Cancelled,
    NoSpace {
        needed: u64,
        available: u64,
    },
    /// The server answered with an error status.
    Http(String),
    /// The connection failed, dropped or stalled; worth another try.
    Network(String),
    Checksum(String),
    Io(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Download cancelled"),
            Self::NoSpace { needed, available } => write!(
                f,
                "Not enough disk space: {:.0} MB needed, {:.0} MB free",
                *needed as f64 / 1e6,
                *available as f64 / 1e6
            ),
            Self::Http(e) => write!(f, "Download failed: {e}"),
            Self::Network(e) => write!(
                f,
                "Download failed: {e}. Check your internet connection and try again; the download continues where it stopped."
            ),
            Self::Checksum(name) => write!(f, "{name} failed its checksum. Try again."),
            Self::Io(e) => write!(f, "Could not write the download: {e}"),
        }
    }
}

fn io(e: std::io::Error) -> DownloadError {
    DownloadError::Io(e.to_string())
}

pub fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// SHA-256 of a file on disk, hex.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Free bytes on the volume holding `dir`, if the OS reports it.
pub fn free_space(dir: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: statvfs writes into `st`, a correctly sized local.
        let rc = unsafe { libc::statvfs(c.as_ptr(), &mut st) };
        (rc == 0).then(|| st.f_bavail as u64 * st.f_frsize as u64)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free = 0u64;
        // SAFETY: NUL-terminated path; out-parameter is a local.
        unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None) }.ok()?;
        Some(free)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = dir;
        None
    }
}

pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("LocalYapper/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }

    /// For tests against a local server: no proxy.
    pub fn without_proxy() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }

    /// Download every file that is not already complete and verified.
    pub async fn fetch_all(
        &self,
        files: &[FileSpec],
        cancel: &AtomicBool,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<(), DownloadError> {
        let total: u64 = files.iter().map(|f| f.size).sum();
        let on_disk = |f: &FileSpec| -> u64 {
            if std::fs::metadata(&f.dest).is_ok_and(|m| m.len() == f.size) {
                f.size
            } else {
                std::fs::metadata(part_path(&f.dest))
                    .map(|m| m.len().min(f.size))
                    .unwrap_or(0)
            }
        };
        let have: u64 = files.iter().map(on_disk).sum();
        if let Some(dir) = files.first().and_then(|f| f.dest.parent()) {
            std::fs::create_dir_all(dir).map_err(io)?;
            let needed = total.saturating_sub(have);
            if let Some(available) = free_space(dir) {
                // Keep 50 MB of headroom.
                if available < needed + 50_000_000 {
                    return Err(DownloadError::NoSpace { needed, available });
                }
            }
        }
        let mut done_before = 0u64;
        for (i, f) in files.iter().enumerate() {
            let base = done_before;
            let report = |file_bytes: u64| {
                progress(Progress {
                    file_index: i,
                    files: files.len(),
                    downloaded: base + file_bytes,
                    total,
                })
            };
            let mut attempt = 0;
            loop {
                match self.fetch_one(f, cancel, &report).await {
                    Err(DownloadError::Network(e)) if attempt < NETWORK_RETRIES => {
                        attempt += 1;
                        log::warn!(
                            "download: {e}; retry {attempt} of {NETWORK_RETRIES} resumes the part"
                        );
                        wait(Duration::from_secs(1 << (attempt - 1)), cancel).await?;
                    }
                    other => break other?,
                }
            }
            done_before += f.size;
            report(f.size);
        }
        Ok(())
    }

    async fn fetch_one(
        &self,
        f: &FileSpec,
        cancel: &AtomicBool,
        report: &(dyn Fn(u64) + Sync),
    ) -> Result<(), DownloadError> {
        let name = f
            .dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // A finished file with the right size and hash needs nothing.
        if std::fs::metadata(&f.dest).is_ok_and(|m| m.len() == f.size)
            && sha256_file(&f.dest).map_err(io)? == f.sha256
        {
            return Ok(());
        }
        let part = part_path(&f.dest);
        // Two passes at most: a corrupt part is thrown away once.
        for _ in 0..2 {
            let mut have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            if have > f.size {
                std::fs::remove_file(&part).map_err(io)?;
                have = 0;
            }
            let mut hasher = Sha256::new();
            if have > 0 {
                let mut r = std::fs::File::open(&part).map_err(io)?;
                let mut buf = vec![0u8; 1 << 16];
                loop {
                    let n = r.read(&mut buf).map_err(io)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
            }
            let mut req = self.client.get(&f.url);
            if have > 0 {
                req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
            }
            let mut resp = timed(req.send(), CONNECT_TIMEOUT, cancel)
                .await?
                .map_err(|e| DownloadError::Network(describe(&e)))?;
            let status = resp.status();
            let mut out = if status == reqwest::StatusCode::PARTIAL_CONTENT && have > 0 {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(&part)
                    .map_err(io)?
            } else if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                // The server says the part already holds every byte.
                if have == f.size && hex(&hasher.finalize()) == f.sha256 {
                    std::fs::rename(&part, &f.dest).map_err(io)?;
                    return Ok(());
                }
                std::fs::remove_file(&part).map_err(io)?;
                continue;
            } else if status.is_success() {
                // 200: the range was ignored; start from zero.
                have = 0;
                hasher = Sha256::new();
                std::fs::File::create(&part).map_err(io)?
            } else {
                return Err(DownloadError::Http(format!("{name}: HTTP {status}")));
            };
            report(have);
            loop {
                // Whatever arrived so far stays in the part file, so a
                // cancelled, stalled or dropped download resumes from it.
                let chunk = match timed(resp.chunk(), STALL_TIMEOUT, cancel).await {
                    Ok(Ok(chunk)) => chunk,
                    Ok(Err(e)) => {
                        let _ = out.flush();
                        return Err(DownloadError::Network(describe(&e)));
                    }
                    Err(e) => {
                        let _ = out.flush();
                        return Err(e);
                    }
                };
                let Some(chunk) = chunk else { break };
                out.write_all(&chunk).map_err(io)?;
                hasher.update(&chunk);
                have += chunk.len() as u64;
                if have > f.size {
                    break;
                }
                report(have);
            }
            out.flush().map_err(io)?;
            drop(out);
            if have < f.size {
                // The server closed early. The part is good as far as it
                // goes, so it is kept and the caller resumes it.
                return Err(DownloadError::Network(
                    "the download ended before the file was complete".into(),
                ));
            }
            if have == f.size && hex(&hasher.finalize()) == f.sha256 {
                std::fs::rename(&part, &f.dest).map_err(io)?;
                return Ok(());
            }
            // Wrong size or hash: discard and try once more from zero.
            let _ = std::fs::remove_file(&part);
        }
        Err(DownloadError::Checksum(name))
    }
}

/// A transport error in plain words.
fn describe(e: &reqwest::Error) -> String {
    if e.is_connect() {
        "could not connect to the download server".into()
    } else if e.is_timeout() {
        "the download server did not answer in time".into()
    } else {
        format!("the connection was interrupted ({})", root_cause(e))
    }
}

/// The innermost cause is the readable part ("connection reset"); the
/// outer layers repeat the long URL.
fn root_cause(e: &reqwest::Error) -> String {
    let mut source: &dyn std::error::Error = e;
    while let Some(next) = source.source() {
        source = next;
    }
    source.to_string()
}

/// Await `fut`, giving up when `cancel` is set or after `limit`.
async fn timed<T>(
    fut: impl std::future::Future<Output = T>,
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<T, DownloadError> {
    tokio::pin!(fut);
    let mut waited = Duration::ZERO;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(DownloadError::Cancelled);
        }
        match tokio::time::timeout(CANCEL_POLL, &mut fut).await {
            Ok(v) => return Ok(v),
            Err(_) => {
                waited += CANCEL_POLL;
                if waited >= limit {
                    return Err(DownloadError::Network(
                        "the download server stopped responding".into(),
                    ));
                }
            }
        }
    }
}

/// Sleep for `d` unless cancelled first.
async fn wait(d: Duration, cancel: &AtomicBool) -> Result<(), DownloadError> {
    match timed(tokio::time::sleep(d), d + CANCEL_POLL * 2, cancel).await {
        Ok(()) => Ok(()),
        Err(DownloadError::Cancelled) => Err(DownloadError::Cancelled),
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// What the test server does with a request.
    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        /// Honour Range with 206.
        Ranges,
        /// Ignore Range and send 200 with the whole body.
        IgnoreRange,
        /// Answer 416 to any Range request.
        Always416,
        /// Send a body with one byte flipped.
        Corrupt,
        /// Drop the connection halfway through the first reply, then
        /// honour Range.
        DropOnce,
        /// Send the headers and half the body, then go quiet.
        Stall,
    }

    struct Server {
        url: String,
        requests: Arc<Mutex<Vec<String>>>,
    }

    fn serve(body: Vec<u8>, mode: Mode) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/file.bin", listener.local_addr().expect("addr"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        std::thread::spawn(move || {
            let mut served = 0usize;
            let mut held = Vec::new();
            for stream in listener.incoming().flatten() {
                served += 1;
                let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                let mut range: Option<u64> = None;
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok() && line != "\r\n" && !line.is_empty() {
                    let lower = line.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("range: bytes=") {
                        range = v.trim().trim_end_matches('-').parse().ok();
                    }
                    line.clear();
                }
                if let Ok(mut l) = log.lock() {
                    l.push(range.map_or("full".into(), |r| format!("from {r}")));
                }
                let mut out = stream;
                let mut data = body.clone();
                if mode == Mode::Corrupt {
                    if let Some(b) = data.get_mut(10) {
                        *b ^= 0xFF;
                    }
                }
                if mode == Mode::Stall || (mode == Mode::DropOnce && served == 1) {
                    let start = range.unwrap_or(0) as usize;
                    let half = start + (data.len() - start) / 2;
                    let head = format!(
                        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        if range.is_some() {
                            "206 Partial Content"
                        } else {
                            "200 OK"
                        },
                        data.len() - start
                    );
                    let _ = out.write_all(head.as_bytes());
                    let _ = out.write_all(&data[start..half]);
                    let _ = out.flush();
                    if mode == Mode::Stall {
                        // Keep the socket open and silent.
                        held.push(out);
                    }
                    continue;
                }
                let resp = match (mode, range) {
                    (Mode::Always416, Some(_)) => {
                        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .as_bytes()
                            .to_vec()
                    }
                    (Mode::Ranges | Mode::DropOnce, Some(r)) => {
                        let slice = &data[r as usize..];
                        let mut v = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            slice.len()
                        )
                        .into_bytes();
                        v.extend_from_slice(slice);
                        v
                    }
                    _ => {
                        let mut v = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            data.len()
                        )
                        .into_bytes();
                        v.extend_from_slice(&data);
                        v
                    }
                };
                let _ = out.write_all(&resp);
            }
        });
        Server { url, requests }
    }

    fn body() -> Vec<u8> {
        (0..200_000u32).map(|i| (i * 31 % 251) as u8).collect()
    }

    fn sha(data: &[u8]) -> String {
        hex(&Sha256::digest(data))
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ly-dl-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        d.join("file.bin")
    }

    fn spec(url: &str, data: &[u8], dest: PathBuf) -> FileSpec {
        FileSpec {
            url: url.into(),
            size: data.len() as u64,
            sha256: sha(data),
            dest,
        }
    }

    fn run(f: &FileSpec, cancel: &AtomicBool) -> Result<Vec<Progress>, DownloadError> {
        let seen = Mutex::new(Vec::new());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let dl = Downloader::without_proxy().expect("client");
        rt.block_on(dl.fetch_all(std::slice::from_ref(f), cancel, &|p| {
            if let Ok(mut s) = seen.lock() {
                s.push(p);
            }
        }))?;
        Ok(seen.into_inner().unwrap_or_default())
    }

    #[test]
    fn downloads_verifies_and_reports_real_totals() {
        let data = body();
        let server = serve(data.clone(), Mode::Ranges);
        let f = spec(&server.url, &data, tmp("plain"));
        let progress = run(&f, &AtomicBool::new(false)).expect("download");
        assert_eq!(std::fs::read(&f.dest).expect("read"), data);
        assert!(!part_path(&f.dest).exists());
        let last = progress.last().copied().unwrap_or_default();
        assert_eq!((last.downloaded, last.total), (200_000, 200_000));
        // A second run finds the file and makes no request.
        let before = server.requests.lock().map(|r| r.len()).unwrap_or(0);
        run(&f, &AtomicBool::new(false)).expect("noop");
        assert_eq!(server.requests.lock().map(|r| r.len()).unwrap_or(0), before);
    }

    #[test]
    fn resumes_a_partial_file_with_range() {
        let data = body();
        let server = serve(data.clone(), Mode::Ranges);
        let f = spec(&server.url, &data, tmp("resume"));
        std::fs::write(part_path(&f.dest), &data[..120_000]).expect("part");
        run(&f, &AtomicBool::new(false)).expect("resume");
        assert_eq!(std::fs::read(&f.dest).expect("read"), data);
        assert_eq!(
            server
                .requests
                .lock()
                .map(|r| r.clone())
                .unwrap_or_default(),
            vec!["from 120000".to_string()]
        );
    }

    #[test]
    fn a_200_reply_to_a_range_restarts_from_zero() {
        let data = body();
        let server = serve(data.clone(), Mode::IgnoreRange);
        let f = spec(&server.url, &data, tmp("restart"));
        std::fs::write(part_path(&f.dest), &data[..50_000]).expect("part");
        run(&f, &AtomicBool::new(false)).expect("restart");
        assert_eq!(std::fs::read(&f.dest).expect("read"), data);
    }

    #[test]
    fn a_416_verifies_what_is_on_disk() {
        let data = body();
        let server = serve(data.clone(), Mode::Always416);
        let f = spec(&server.url, &data, tmp("416"));
        std::fs::write(part_path(&f.dest), &data).expect("full part");
        run(&f, &AtomicBool::new(false)).expect("416 complete");
        assert_eq!(std::fs::read(&f.dest).expect("read"), data);
    }

    #[test]
    fn corruption_is_rejected_and_nothing_is_installed() {
        let data = body();
        let server = serve(data.clone(), Mode::Corrupt);
        let f = spec(&server.url, &data, tmp("corrupt"));
        let err = run(&f, &AtomicBool::new(false)).expect_err("checksum");
        assert!(matches!(err, DownloadError::Checksum(_)), "{err:?}");
        assert!(!f.dest.exists());
        assert!(!part_path(&f.dest).exists());
    }

    #[test]
    fn a_dropped_connection_resumes_by_itself() {
        let data = body();
        let server = serve(data.clone(), Mode::DropOnce);
        let f = spec(&server.url, &data, tmp("drop"));
        run(&f, &AtomicBool::new(false)).expect("retried");
        assert_eq!(std::fs::read(&f.dest).expect("read"), data);
        // The retry asked only for the missing half.
        assert_eq!(
            server
                .requests
                .lock()
                .map(|r| r.clone())
                .unwrap_or_default(),
            vec!["full".to_string(), "from 100000".to_string()]
        );
    }

    #[test]
    fn cancel_is_noticed_while_the_server_is_silent() {
        let data = body();
        let server = serve(data.clone(), Mode::Stall);
        let f = spec(&server.url, &data, tmp("stall"));
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            flag.store(true, Ordering::Release);
        });
        let started = std::time::Instant::now();
        let err = run(&f, &cancel).expect_err("cancelled");
        assert_eq!(err, DownloadError::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(5));
        // What arrived before the silence is kept for the next attempt.
        assert_eq!(
            std::fs::metadata(part_path(&f.dest)).map(|m| m.len()).ok(),
            Some(100_000)
        );
    }

    #[test]
    fn cancel_keeps_the_part() {
        let data = body();
        let server = serve(data.clone(), Mode::Ranges);
        let f = spec(&server.url, &data, tmp("cancel"));
        std::fs::write(part_path(&f.dest), &data[..10_000]).expect("part");
        let err = run(&f, &AtomicBool::new(true)).expect_err("cancelled");
        assert_eq!(err, DownloadError::Cancelled);
        assert!(!f.dest.exists());
        assert!(part_path(&f.dest).exists());
        // Resuming afterwards completes the file.
        run(&f, &AtomicBool::new(false)).expect("resume after cancel");
        assert_eq!(std::fs::read(&f.dest).expect("read"), data);
        let _ = server;
    }

    #[test]
    fn free_space_is_reported() {
        assert!(free_space(&std::env::temp_dir()).unwrap_or(0) > 0);
    }
}
