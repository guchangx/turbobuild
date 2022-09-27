use anyhow::Context;

extern crate blake3;

pub struct Digest {
    inner: blake3::Hasher,
}

impl Default for Digest {
    fn default() -> Self {
        Self::new()
    }
}

impl Digest {
    pub fn new() -> Digest {
        Digest {
            inner: blake3::Hasher::new()
        }
    }
    
    pub async fn file(path: std::path::PathBuf) -> anyhow::Result<String>
    {
        Self::reader(path).await
    }

    pub fn reader_sync<R: std::io::Read>(mut reader: R) -> anyhow::Result<String> {
        let mut digest = Digest::new();
        let mut buffer = [0; 128 * 1024];
        loop {
            let count = reader.read(&mut buffer[..])?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }

        Ok(digest.finish())
    }

    pub async fn reader(path: std::path::PathBuf) -> anyhow::Result<String> {
        let pool = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .unwrap();

        pool.spawn_blocking(move || {
            let reader = std::fs::File::open(&path).with_context(|| format!("Failed to open file for hashing: {:?}", path))?;

            let digest = Self::reader_sync(reader);
            return digest;

         })
        .await?
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    pub fn finish(self) -> String {
        hex(self.inner.finalize().as_bytes())
    }
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        s.push(hex(byte & 0xf));
        s.push(hex((byte >> 4) & 0xf));
    }
    return s;

    fn hex(byte: u8) -> char {
        match byte {
            0..=9 => (b'0' + byte) as char,
            _ => (b'a' + byte - 10) as char,
        }
    }
}