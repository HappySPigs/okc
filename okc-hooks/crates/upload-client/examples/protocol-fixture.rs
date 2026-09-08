//! Emits real Rust/serde CBOR for cross-language receiver contract verification.
//! Arguments are zero or more pairs of vault-relative path and UTF-8 content.

use content_core::ContentAddressing;
use foundation::{ByteCount, Manifest, ManifestEntry, RelativePath, encode};
use upload_client::{
    ChunkFrame, CommitRequest, NegotiateRequest, negotiate_entries, path_hash_map,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.len().is_multiple_of(2) {
        return Err("expected PATH CONTENT pairs".into());
    }
    let mut entries = Vec::new();
    let mut blobs = Vec::new();
    for pair in args.chunks_exact(2) {
        let path = RelativePath::normalize(&pair[0])?;
        let bytes = pair[1].as_bytes().to_vec();
        let digest = ContentAddressing::hash_stream(bytes.as_slice())?;
        let size = ByteCount::new(bytes.len() as u64);
        entries.push(ManifestEntry {
            relative_path: path,
            raw_sha256: digest,
            size,
        });
        let frame = ChunkFrame {
            blob: digest,
            offset: ByteCount::new(0),
            len: size,
            chunk_sha256: digest,
            bytes,
        };
        blobs.push(serde_json::json!({"sha256_hex": digest.to_string(), "frame_hex": hex(&encode(&frame)?)}));
    }
    entries.sort();
    let manifest = Manifest {
        manifest_digest: ContentAddressing::manifest_digest(&entries),
        entries,
    };
    let negotiate = NegotiateRequest {
        manifest_digest: manifest.manifest_digest,
        entries: negotiate_entries(&manifest),
    };
    let commit = CommitRequest {
        path_hash_map: path_hash_map(&manifest),
        manifest_digest: manifest.manifest_digest,
    };
    println!(
        "{}",
        serde_json::json!({
            "manifest_digest_hex": manifest.manifest_digest.to_string(),
            "negotiate_hex": hex(&encode(&negotiate)?),
            "commit_hex": hex(&encode(&commit)?),
            "blobs": blobs,
        })
    );
    Ok(())
}
