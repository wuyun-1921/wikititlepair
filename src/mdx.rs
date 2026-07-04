use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;

const KEY_BLOCK_SZ: usize = 32768;
const REC_BLOCK_SZ: usize = 65536;

struct KeyBlock {
    comp: Vec<u8>,
    n_entries: usize,
    first_key: Vec<u8>,  // null-terminated
    last_key: Vec<u8>,   // null-terminated
}

/// Write MDX v2 dictionary directly from sorted (key, html_body) entries.
/// Memory: Vec<u64> offset table (8 bytes/entry) + compressed blocks.
pub fn write_mdx(
    path: &Path,
    title: &str,
    description: &str,
    entries: &[(String, String)],
) -> std::io::Result<()> {
    let n = entries.len();
    let mut offsets: Vec<u64> = Vec::with_capacity(n);

    // ---- Pass 1: build record blocks, track per-entry offsets ----
    let mut rec_compressed: Vec<Vec<u8>> = Vec::new();
    let mut rec_index: Vec<(u64, u64)> = Vec::new();
    {
        let mut buf = Vec::with_capacity(REC_BLOCK_SZ);
        let mut off: u64 = 0;
        for (_, body) in entries.iter() {
            let b = body.as_bytes();
            let elen = b.len() + 1;
            if buf.len() + elen > REC_BLOCK_SZ && !buf.is_empty() {
                rec_index.push((0, buf.len() as u64));
                let c = compress_block(&buf);
                rec_index.last_mut().unwrap().0 = c.len() as u64;
                rec_compressed.push(c);
                buf.clear();
            }
            offsets.push(off);
            buf.extend_from_slice(b);
            buf.push(0);
            off += elen as u64;
        }
        if !buf.is_empty() {
            rec_index.push((0, buf.len() as u64));
            let c = compress_block(&buf);
            rec_index.last_mut().unwrap().0 = c.len() as u64;
            rec_compressed.push(c);
        }
    }

    // ---- Pass 2: build key blocks, tracking first/last keys explicitly ----
    let mut key_blocks: Vec<KeyBlock> = Vec::new();
    {
        let mut buf = Vec::with_capacity(KEY_BLOCK_SZ);
        let mut n_in_block: usize = 0;
        let mut first_key: Option<Vec<u8>> = None;
        let mut last_key: Vec<u8> = Vec::new();

        for (i, (key, _)) in entries.iter().enumerate() {
            let k = key.as_bytes();
            let elen = 8 + k.len() + 1;
            if buf.len() + elen > KEY_BLOCK_SZ && !buf.is_empty() {
                let c = compress_block(&buf);
                key_blocks.push(KeyBlock {
                    comp: c,
                    n_entries: n_in_block,
                    first_key: first_key.take().unwrap_or_default(),
                    last_key: std::mem::take(&mut last_key),
                });
                buf.clear();
                n_in_block = 0;
            }
            if first_key.is_none() {
                let mut fk = Vec::with_capacity(k.len() + 1);
                fk.extend_from_slice(k);
                fk.push(0);
                first_key = Some(fk);
            }
            // Always update last_key
            last_key.clear();
            last_key.extend_from_slice(k);
            last_key.push(0);

            buf.extend_from_slice(&offsets[i].to_be_bytes());
            buf.extend_from_slice(k);
            buf.push(0);
            n_in_block += 1;
        }
        if !buf.is_empty() {
            let c = compress_block(&buf);
            key_blocks.push(KeyBlock {
                comp: c,
                n_entries: n_in_block,
                first_key: first_key.unwrap_or_default(),
                last_key,
            });
        }
    }

    // ---- Build key block index (decompressed, then compressed for v2) ----
    let mut key_index_decomp = Vec::new();
    for kb in &key_blocks {
        let fk = &kb.first_key;
        let lk = &kb.last_key;
        key_index_decomp.extend_from_slice(&(kb.n_entries as u64).to_be_bytes());
        key_index_decomp.extend_from_slice(&((fk.len() - 1) as u16).to_be_bytes());
        key_index_decomp.extend_from_slice(fk);
        key_index_decomp.extend_from_slice(&((lk.len() - 1) as u16).to_be_bytes());
        key_index_decomp.extend_from_slice(lk);
        key_index_decomp.extend_from_slice(&(kb.comp.len() as u64).to_be_bytes());
        // decompressed size: approximate (not critical for correctness)
        key_index_decomp.extend_from_slice(&0u64.to_be_bytes());
    }
    let key_index_comp = compress_block(&key_index_decomp);

    // ---- Write file ----
    let mut out = BufWriter::new(File::create(path)?);

    // Header
    let header_xml = format!(
        "<Dictionary \
         GeneratedByEngineVersion=\"2.0\" \
         RequiredEngineVersion=\"2.0\" \
         Encrypted=\"No\" \
         Encoding=\"UTF-8\" \
         Format=\"Html\" \
         Stripkey=\"Yes\" \
         CreationDate=\"2026-07-04\" \
         Compact=\"Yes\" \
         Compat=\"Yes\" \
         KeyCaseSensitive=\"No\" \
         Description=\"{}\" \
         Title=\"{}\" \
         DataSourceFormat=\"106\" \
         StyleSheet=\"\" \
         Left2Right=\"Yes\" \
         RegisterBy=\"\" \
         />\r\n\x00",
        xml_escape(description), xml_escape(title)
    );
    let hdr_utf16: Vec<u8> = header_xml
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let hdr_adler = adler32(&hdr_utf16);

    out.write_all(&(hdr_utf16.len() as i32).to_be_bytes())?;
    out.write_all(&hdr_utf16)?;
    out.write_all(&hdr_adler.to_le_bytes())?;

    // Key section header (v2: 5 values + adler32)
    let key_data_total: u64 = key_blocks.iter().map(|kb| kb.comp.len() as u64).sum();
    {
        let mut h = Vec::with_capacity(40);
        h.extend_from_slice(&(key_blocks.len() as u64).to_be_bytes());
        h.extend_from_slice(&(n as u64).to_be_bytes());
        h.extend_from_slice(&(key_index_decomp.len() as u64).to_be_bytes());
        h.extend_from_slice(&(key_index_comp.len() as u64).to_be_bytes());
        h.extend_from_slice(&key_data_total.to_be_bytes());
        let a = adler32(&h);
        out.write_all(&h)?;
        out.write_all(&a.to_be_bytes())?;
    }
    out.write_all(&key_index_comp)?;

    // Key blocks (adler32 already in compress_block output)
    for kb in &key_blocks {
        out.write_all(&kb.comp)?;
    }

    // Record section header (4 values)
    let rec_data_total: u64 = rec_compressed.iter().map(|b| b.len() as u64).sum();
    {
        let ridx_sz = rec_compressed.len() as u64 * 16;
        out.write_all(&(rec_compressed.len() as u64).to_be_bytes())?;
        out.write_all(&(n as u64).to_be_bytes())?;
        out.write_all(&ridx_sz.to_be_bytes())?;
        out.write_all(&rec_data_total.to_be_bytes())?;
    }

    // Record block index (uncompressed)
    for &(cs, ds) in &rec_index {
        out.write_all(&cs.to_be_bytes())?;
        out.write_all(&ds.to_be_bytes())?;
    }

    // Record blocks (adler32 already in compress_block output)
    for block in &rec_compressed {
        out.write_all(block)?;
    }

    out.flush()?;
    Ok(())
}

/// MDX block format: [u32 LE: 2 (zlib)] [u32 BE: adler32 of decomp] [zlib data]
fn compress_block(decomp: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + decomp.len() / 2);
    // Info: compression_type = 2 (zlib), no encryption
    out.extend_from_slice(&2u32.to_le_bytes());
    // Adler32 of decompressed data, BigEndian
    let adler = adler32(decomp);
    out.extend_from_slice(&adler.to_be_bytes());
    // Zlib-compressed data
    let mut enc = ZlibEncoder::new(&mut out, Compression::default());
    enc.write_all(decomp).ok();
    enc.finish().ok();
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_mdx() {
        let entries = vec![
            ("hello".to_string(), "<b>hello</b>".to_string()),
            ("world".to_string(), "<b>world</b>".to_string()),
            ("test".to_string(), "<i>test</i>".to_string()),
        ];
        let path = std::env::temp_dir().join("test_small.mdx");
        write_mdx(&path, "TestDict", "A test dictionary", &entries).unwrap();
        assert!(path.exists());
        eprintln!("MDX size: {} bytes", path.metadata().unwrap().len());

        // Verify with mdict-utils query
        let output = std::process::Command::new("mdict")
            .args(["-q", "hello", path.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("Query 'hello': {}", stdout);
        assert!(output.status.success(), "mdict query failed: {:?}", String::from_utf8_lossy(&output.stderr));
        assert!(stdout.contains("<b>hello</b>"), "expected <b>hello</b> in output");

        // Query case-insensitive
        let output2 = std::process::Command::new("mdict")
            .args(["-q", "HELLO", path.to_str().unwrap()])
            .output()
            .unwrap();
        // MDict is case-insensitive by default
        eprintln!("Query 'HELLO': {}", String::from_utf8_lossy(&output2.stdout));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_medium_mdx() {
        // ~1000 entries — enough to span multiple record blocks
        let mut entries = Vec::new();
        for i in 0..1000 {
            let key = format!("word-{:04}", i);
            entries.push((
                key.clone(),
                format!("<span>Definition for {}</span>", key),
            ));
        }
        let path = std::env::temp_dir().join("test_medium.mdx");
        write_mdx(&path, "Medium", "Medium test dict", &entries).unwrap();
        let size = path.metadata().unwrap().len();
        eprintln!("Medium MDX size: {} bytes", size);
        assert!(size > 1000);

        // Query a few entries and verify correct content
        for key in &["word-0000", "word-0500", "word-0999"] {
            let output = std::process::Command::new("mdict")
                .args(["-q", key, path.to_str().unwrap()])
                .output()
                .unwrap();
            assert!(output.status.success(),
                "query '{}' failed: {:?}", key, String::from_utf8_lossy(&output.stderr));
            let stdout = String::from_utf8_lossy(&output.stdout);
            let expected = format!("<span>Definition for {}</span>", key);
            assert!(stdout.contains(&expected),
                "query '{}' returned wrong content: {:?}", key, stdout);
        }

        let _ = std::fs::remove_file(&path);
    }
}
