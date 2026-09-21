//! Fetching a file over the machine's own HTTP stack.
//!
//! A setup that has to bring along something it cannot ship inside itself -- a
//! runtime the product depends on, a payload that is too large to bundle --
//! asks WinHTTP for it. That is the HTTP stack every supported Windows already
//! carries, so the machine's proxy settings, its certificate store and its TLS
//! configuration are the ones a browser on that machine would use, and the
//! product adds no client library of its own.
//!
//! What arrives is not trusted because it arrived: the caller names the SHA-256
//! it expects, the bytes are hashed while they are streamed, and a file that
//! does not match never stays on the disk.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetOption, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_FLAG_REFRESH, WINHTTP_FLAG_SECURE,
    WINHTTP_FLAG_SECURE_PROTOCOL_TLS1, WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_1,
    WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS,
    WINHTTP_OPTION_SECURE_PROTOCOLS, WINHTTP_QUERY_CONTENT_LENGTH, WINHTTP_QUERY_FLAG_NUMBER,
    WINHTTP_QUERY_STATUS_CODE,
};
use windows::Win32::Security::Cryptography::{
    CryptAcquireContextW, CryptCreateHash, CryptDestroyHash, CryptGetHashParam, CryptHashData,
    CryptReleaseContext, CALG_SHA_256, CRYPT_VERIFYCONTEXT, HP_HASHVAL, MS_ENH_RSA_AES_PROV,
    PROV_RSA_AES,
};

use crate::install::{check_cancelled, wide, Cancellation};

/// What every request says it comes from, so a server's logs name the product
/// rather than an anonymous client.
const USER_AGENT: &str = concat!("nano-installer/", env!("CARGO_PKG_VERSION"));

/// How much of a response is read at a time.
const CHUNK: usize = 64 * 1024;

/// Fetches `url` into `target`, checking the SHA-256 when one is named.
///
/// `on_progress` is told how many bytes have arrived and, when the server
/// announced a length, how many are coming. A download the user cancels, a
/// download that fails, and a download whose hash does not match all leave the
/// target absent: the step that would have used the file must not be able to
/// run against half of it.
pub(super) fn download(
    url: &str,
    target: &Path,
    expected_sha256: Option<&str>,
    on_progress: &mut dyn FnMut(u64, Option<u64>),
    task: &Cancellation,
) -> Result<()> {
    let outcome = fetch(url, target, expected_sha256, on_progress, task);
    if outcome.is_err() {
        let _ = std::fs::remove_file(target);
    }
    outcome
}

/// The SHA-256 of a file, in lower-case hexadecimal.
pub(super) fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("cannot read {} to check its SHA-256", path.display()))?;
    let mut hasher = Sha256::new()?;
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read])?;
    }
    hasher.finish()
}

/// The SHA-256 of `bytes`, in lower-case hexadecimal.
///
/// A file is what a download is checked against, so nothing outside the tests
/// hashes bytes that are already in memory.
#[cfg(test)]
pub(super) fn sha256_hex(bytes: &[u8]) -> Result<String> {
    let mut hasher = Sha256::new()?;
    hasher.update(bytes)?;
    hasher.finish()
}

fn fetch(
    url: &str,
    target: &Path,
    expected_sha256: Option<&str>,
    on_progress: &mut dyn FnMut(u64, Option<u64>),
    task: &Cancellation,
) -> Result<()> {
    let endpoint = endpoint(url)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    let agent = wide(USER_AGENT);
    let session = Handle::open(
        unsafe {
            WinHttpOpen(
                PCWSTR(agent.as_ptr()),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            )
        },
        "starting an HTTP session",
    )?;
    let host = wide(&endpoint.host);
    let connection = Handle::open(
        unsafe { WinHttpConnect(session.0, PCWSTR(host.as_ptr()), endpoint.port, 0) },
        &format!("connecting to {}", endpoint.host),
    )?;
    let verb = wide("GET");
    let object = wide(&endpoint.path);
    let secure = if endpoint.secure {
        WINHTTP_FLAG_SECURE.0
    } else {
        0
    };
    let request = Handle::open(
        unsafe {
            WinHttpOpenRequest(
                connection.0,
                PCWSTR(verb.as_ptr()),
                PCWSTR(object.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_OPEN_REQUEST_FLAGS(secure | WINHTTP_FLAG_REFRESH.0),
            )
        },
        &format!("preparing the request for {url}"),
    )?;
    // A release is normally announced behind a redirect, so the request follows
    // one rather than reporting the server that answered first.
    let policy = WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS.to_ne_bytes();
    unsafe {
        WinHttpSetOption(
            Some(request.0),
            WINHTTP_OPTION_REDIRECT_POLICY,
            Some(&policy),
        )
    }
    .map_err(|_| last_error("setting the redirect policy"))?;
    if endpoint.secure {
        // WinHTTP's own default on the older systems this product supports can
        // leave out TLS 1.2, which is what most download hosts now require.
        let protocols: u32 = WINHTTP_FLAG_SECURE_PROTOCOL_TLS1
            | WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_1
            | WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2;
        unsafe {
            WinHttpSetOption(
                Some(request.0),
                WINHTTP_OPTION_SECURE_PROTOCOLS,
                Some(&protocols.to_ne_bytes()),
            )
        }
        .map_err(|_| last_error("enabling TLS"))?;
    }
    unsafe { WinHttpSendRequest(request.0, None, None, 0, 0, 0) }
        .map_err(|_| last_error(&format!("asking for {url}")))?;
    unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) }
        .map_err(|_| last_error(&format!("reading the answer from {url}")))?;

    let status = query_u32(request.0, WINHTTP_QUERY_STATUS_CODE)?;
    if !(200..300).contains(&status) {
        bail!("{url} answered HTTP {status}");
    }
    let announced = query_u32(request.0, WINHTTP_QUERY_CONTENT_LENGTH)
        .ok()
        .filter(|length| *length > 0)
        .map(u64::from);

    let mut file = std::fs::File::create(target)
        .with_context(|| format!("cannot write {}", target.display()))?;
    let mut hasher = Sha256::new()?;
    let mut buffer = vec![0u8; CHUNK];
    let mut read_total = 0u64;
    loop {
        check_cancelled(task)?;
        let mut available = 0u32;
        if unsafe { WinHttpQueryDataAvailable(request.0, &mut available) }.is_err() {
            return Err(last_error(&format!("reading {url}")));
        }
        if available == 0 {
            break;
        }
        let wanted = available.min(CHUNK as u32);
        let mut read = 0u32;
        if unsafe { WinHttpReadData(request.0, buffer.as_mut_ptr().cast(), wanted, &mut read) }
            .is_err()
        {
            return Err(last_error(&format!("reading {url}")));
        }
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read as usize];
        file.write_all(chunk)
            .with_context(|| format!("cannot write {}", target.display()))?;
        hasher.update(chunk)?;
        read_total += u64::from(read);
        on_progress(read_total, announced);
    }
    file.flush()?;
    drop(file);
    if let Some(announced) = announced {
        if read_total != announced {
            bail!("{url} stopped after {read_total} of {announced} bytes");
        }
    }
    let digest = hasher.finish()?;
    if let Some(expected) = expected_sha256 {
        if !expected.trim().eq_ignore_ascii_case(&digest) {
            bail!("{url} arrived as sha256 {digest}, but the project expects {expected}");
        }
    }
    Ok(())
}

/// A URL split into the parts a WinHTTP request asks for.
struct Endpoint {
    secure: bool,
    host: String,
    port: u16,
    path: String,
}

fn endpoint(url: &str) -> Result<Endpoint> {
    let (secure, rest) = match url.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("https") => (true, rest),
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") => (false, rest),
        Some((scheme, _)) => bail!("{url} uses {scheme}, and a download can only be http or https"),
        None => bail!("{url} is not a URL"),
    };
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>()
                .with_context(|| format!("{url} does not carry a port number"))?,
        ),
        None => (authority, if secure { 443 } else { 80 }),
    };
    if host.is_empty() {
        bail!("{url} names no host");
    }
    Ok(Endpoint {
        secure,
        host: host.to_string(),
        port,
        path: path.to_string(),
    })
}

/// An open WinHTTP handle, closed however the request ends.
struct Handle(*mut core::ffi::c_void);

impl Handle {
    fn open(handle: *mut core::ffi::c_void, what: &str) -> Result<Self> {
        if handle.is_null() {
            return Err(last_error(what));
        }
        Ok(Self(handle))
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}

/// The last WinHTTP error, named with what was being attempted.
fn last_error(what: &str) -> anyhow::Error {
    let code = unsafe { GetLastError() };
    anyhow::anyhow!("{what} failed (WinHTTP error {})", code.0)
}

/// Reads a numeric response header, which is how the status and the length come
/// back.
fn query_u32(request: *mut core::ffi::c_void, header: u32) -> Result<u32> {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    unsafe {
        WinHttpQueryHeaders(
            request,
            header | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut value as *mut u32 as *mut core::ffi::c_void),
            &mut size,
            std::ptr::null_mut(),
        )?;
    }
    Ok(value)
}

/// A running SHA-256, backed by the machine's own cryptographic provider.
struct Sha256 {
    provider: usize,
    hash: usize,
}

impl Sha256 {
    fn new() -> Result<Self> {
        let mut provider = 0usize;
        unsafe {
            CryptAcquireContextW(
                &mut provider,
                PCWSTR::null(),
                MS_ENH_RSA_AES_PROV,
                PROV_RSA_AES,
                CRYPT_VERIFYCONTEXT,
            )
        }
        .context("the machine has no SHA-256 provider")?;
        let mut hash = 0usize;
        match unsafe { CryptCreateHash(provider, CALG_SHA_256, 0, 0, &mut hash) } {
            Ok(()) => Ok(Self { provider, hash }),
            Err(error) => {
                unsafe {
                    let _ = CryptReleaseContext(provider, 0);
                }
                Err(error).context("cannot start a SHA-256")
            }
        }
    }

    fn update(&mut self, data: &[u8]) -> Result<()> {
        unsafe { CryptHashData(self.hash, data, 0) }.context("cannot hash the downloaded bytes")
    }

    fn finish(self) -> Result<String> {
        let mut digest = [0u8; 32];
        let mut length = digest.len() as u32;
        let result = unsafe {
            CryptGetHashParam(
                self.hash,
                HP_HASHVAL.0,
                Some(digest.as_mut_ptr()),
                &mut length,
                0,
            )
        };
        if let Err(error) = result {
            return Err(error).context("cannot finish the SHA-256");
        }
        let digest = &digest[..(length as usize).min(digest.len())];
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }
}

impl Drop for Sha256 {
    fn drop(&mut self) {
        unsafe {
            let _ = CryptDestroyHash(self.hash);
            let _ = CryptReleaseContext(self.provider, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_url_into_what_a_request_asks_for() {
        let plain = endpoint("http://example.test/releases/app.zip").expect("a URL");
        assert!(!plain.secure);
        assert_eq!(plain.host, "example.test");
        assert_eq!(plain.port, 80);
        assert_eq!(plain.path, "/releases/app.zip");

        let secure = endpoint("https://example.test").expect("a URL");
        assert!(secure.secure);
        assert_eq!(secure.port, 443);
        assert_eq!(secure.path, "/");

        let stated = endpoint("https://example.test:8443/a/b?c=d").expect("a URL");
        assert_eq!(stated.port, 8443);
        assert_eq!(stated.path, "/a/b?c=d");

        // The scheme is matched however it is written, because a project's
        // configuration is hand-written text.
        assert!(endpoint("HTTPS://example.test/x").expect("a URL").secure);
    }

    #[test]
    fn refuses_a_url_it_cannot_fetch() {
        for url in [
            "ftp://example.test/app.zip",
            "example.test/app.zip",
            "https:///app.zip",
            "https://example.test:not-a-port/app.zip",
        ] {
            assert!(endpoint(url).is_err(), "{url} should be refused");
        }
    }

    #[test]
    fn hashes_to_the_published_digest() -> Result<()> {
        // The classic check value: a hasher that returns what the caller wants
        // to hear would pass anything, so the digest of a known answer is what
        // says otherwise.
        assert_eq!(
            sha256_hex(b"abc")?,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"")?,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        Ok(())
    }

    #[test]
    fn hashes_a_file_the_way_it_hashes_the_bytes() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("payload.bin");
        // Larger than one read, so the stream is followed across chunks.
        let contents: Vec<u8> = (0..200_000u32).map(|index| index as u8).collect();
        std::fs::write(&path, &contents)?;
        assert_eq!(sha256_file(&path)?, sha256_hex(&contents)?);
        Ok(())
    }
}
