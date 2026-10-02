//! The accelerated backend keeps the payload and streamed value addresses of the previous
//! SHA-256 implementation, including padding and encoder-window boundaries.
use ekr_core::{Canonical, ContentHash, Encoder};
use sha2::{Digest, Sha256};

struct Chunks<'a> {
    bytes: &'a [u8],
    width: usize,
}
impl Canonical for Chunks<'_> {
    fn encode(&self, out: &mut Encoder) {
        for chunk in self.bytes.chunks(self.width) {
            out.bytes(chunk);
        }
    }
}

fn reference(domain: &[u8], bytes: &[u8]) -> ContentHash {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(bytes);
    ContentHash::from_bytes(digest.finalize().into())
}

#[test]
fn payload_and_streamed_value_digests_keep_every_boundary() {
    for size in [
        0, 1, 42, 43, 44, 55, 56, 63, 64, 65, 65535, 65536, 65537, 131073,
    ] {
        let bytes: Vec<_> = (0..size)
            .map(|at| u8::try_from(at % 251).unwrap())
            .collect();
        assert_eq!(
            ContentHash::of_bytes(&bytes),
            reference(ContentHash::PAYLOAD_DOMAIN, &bytes)
        );
        for width in [1, 55, 64, 256, 65536, 65537] {
            let value = Chunks {
                bytes: &bytes,
                width,
            };
            assert_eq!(
                ContentHash::of(&value),
                reference(ContentHash::VALUE_DOMAIN, &value.canonical_bytes()),
                "size={size} width={width}"
            );
        }
    }
}
