//! The viewer page's pinned graph libraries, served to a test's browser from bytes this process
//! holds, so that no browser a test starts reaches the network
//! (`story:browser-tests-need-no-network`).
//!
//! The page (`src/cli/viewer/index.html`) loads its libraries from [`HOST`], each by a
//! `<script src=… integrity="sha384-…" crossorigin="anonymous">` tag. The bytes are committed under
//! `tests/fixtures/viewer-libraries/`, at the path of their address on that host, beside each
//! package's licence. A harness that drives its browser over the DevTools protocol:
//!
//! - starts the browser with [`UNRESOLVABLE`], so nothing that escaped interception could be
//!   answered from the network;
//! - calls [`serve`] once connected, which pauses every request to [`HOST`] (`Fetch.enable`);
//! - hands every `Fetch.requestPaused` it reads to [`answer`], which fulfils a script the page names
//!   from its fixture and fails any other address on that host with `BlockedByClient`.
//!
//! The page and its Content-Security-Policy are unchanged: the browser still requests each script
//! at its pinned address and still checks it against its `integrity`.

// Six test binaries include this file and each uses a part of it.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use base64::Engine as _;
use serde_json::{json, Value};

/// The host the page loads its libraries from.
pub(crate) const HOST: &str = "https://cdn.jsdelivr.net/";

/// The browser flag that leaves [`HOST`] unresolvable for the browser.
pub(crate) const UNRESOLVABLE: &str = "--host-resolver-rules=MAP cdn.jsdelivr.net ~NOTFOUND";

/// The directory the fixtures live in, relative to the crate.
pub(crate) const FIXTURES: &str = "tests/fixtures/viewer-libraries";

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
}

/// One script the page loads from elsewhere: its address and its `integrity` attribute, empty when
/// the tag has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pinned {
    pub(crate) url: String,
    pub(crate) integrity: String,
}

/// Every `<script>` tag of `page` that has a `src`, in order.
pub(crate) fn pinned(page: &str) -> Vec<Pinned> {
    page.split("<script")
        .skip(1)
        .filter_map(|rest| {
            let tag = &rest[..rest.find('>')?];
            Some(Pinned {
                url: attribute(tag, "src")?,
                integrity: attribute(tag, "integrity").unwrap_or_default(),
            })
        })
        .collect()
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let at = tag.find(&needle)? + needle.len();
    Some(tag[at..].split('"').next()?.to_owned())
}

/// The scripts the embedded page loads, read from `src/cli/viewer/index.html` once per process.
pub(crate) fn page_pinned() -> &'static [Pinned] {
    static PINNED: OnceLock<Vec<Pinned>> = OnceLock::new();
    PINNED.get_or_init(|| {
        let page = std::fs::read_to_string(manifest_dir().join("src/cli/viewer/index.html"))
            .expect("the embedded page");
        pinned(&page)
    })
}

/// Where the fixture of `url` lives: its path on [`HOST`] under [`FIXTURES`]. `None` for another
/// host, or a path with an empty, `.` or `..` segment, a query or a fragment.
pub(crate) fn fixture_path(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix(HOST)?;
    if path.contains(['?', '#', '\\'])
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return None;
    }
    Some(manifest_dir().join(FIXTURES).join(path))
}

/// The bytes served for `url`: its fixture, when the embedded page loads that address; `None`
/// otherwise.
pub(crate) fn fixture(url: &str) -> Option<Vec<u8>> {
    if !page_pinned().iter().any(|pinned| pinned.url == url) {
        return None;
    }
    std::fs::read(fixture_path(url)?).ok()
}

/// `Fetch.enable`'s parameters: every request to [`HOST`] paused before it is sent, and the
/// harness's own `others` besides. Enabling again replaces the patterns, so a harness that pauses
/// requests of its own passes them here rather than enabling without [`HOST`].
pub(crate) fn patterns_with(others: &[Value]) -> Value {
    let mut patterns = vec![json!({"urlPattern": format!("{HOST}*"), "requestStage": "Request"})];
    patterns.extend(others.iter().cloned());
    json!({ "patterns": patterns })
}

/// Pauses every request to [`HOST`], through the harness's own protocol `call`, before the page is
/// opened.
pub(crate) fn serve<T>(call: impl FnOnce(&str, Value) -> T) -> T {
    call("Fetch.enable", patterns_with(&[]))
}

/// The command, and its parameters, that answers a `Fetch.requestPaused` event's `params` from the
/// fixtures. `None` for a request to another host, which the harness answers itself.
pub(crate) fn answer(paused: &Value) -> Option<(&'static str, Value)> {
    answer_from(paused, fixture)
}

/// [`answer`], with the bytes for an address taken from `body`: a fulfilment when it has some, a
/// `BlockedByClient` failure when it has none.
pub(crate) fn answer_from(
    paused: &Value,
    body: impl Fn(&str) -> Option<Vec<u8>>,
) -> Option<(&'static str, Value)> {
    let url = paused["request"]["url"].as_str()?;
    if !url.starts_with(HOST) {
        return None;
    }
    let id = paused["requestId"].clone();
    Some(match body(url) {
        Some(bytes) => ("Fetch.fulfillRequest", fulfilment(&id, &bytes)),
        None => (
            "Fetch.failRequest",
            json!({"requestId": id, "errorReason": "BlockedByClient"}),
        ),
    })
}

/// A `200` carrying `bytes` as a script any origin may read: the page's tags are
/// `crossorigin="anonymous"`, so the browser checks the answer under CORS before its integrity.
pub(crate) fn fulfilment(request: &Value, bytes: &[u8]) -> Value {
    json!({
        "requestId": request,
        "responseCode": 200,
        "responseHeaders": [
            {"name": "Content-Type", "value": "application/javascript"},
            {"name": "Access-Control-Allow-Origin", "value": "*"},
        ],
        "body": base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

/// `bytes` as an `integrity` attribute names them: `sha384-` and the base64 of their SHA-384.
pub(crate) fn integrity(bytes: &[u8]) -> String {
    format!(
        "sha384-{}",
        base64::engine::general_purpose::STANDARD.encode(sha384(bytes))
    )
}

/// SHA-384 (FIPS 180-4 § 6.5): the SHA-512 computation from its own initial value, truncated to
/// 48 bytes. Held here so the integrity check needs no dependency the crate does not declare.
pub(crate) fn sha384(bytes: &[u8]) -> [u8; 48] {
    const K: [u64; 80] = [
        0x428a_2f98_d728_ae22,
        0x7137_4491_23ef_65cd,
        0xb5c0_fbcf_ec4d_3b2f,
        0xe9b5_dba5_8189_dbbc,
        0x3956_c25b_f348_b538,
        0x59f1_11f1_b605_d019,
        0x923f_82a4_af19_4f9b,
        0xab1c_5ed5_da6d_8118,
        0xd807_aa98_a303_0242,
        0x1283_5b01_4570_6fbe,
        0x2431_85be_4ee4_b28c,
        0x550c_7dc3_d5ff_b4e2,
        0x72be_5d74_f27b_896f,
        0x80de_b1fe_3b16_96b1,
        0x9bdc_06a7_25c7_1235,
        0xc19b_f174_cf69_2694,
        0xe49b_69c1_9ef1_4ad2,
        0xefbe_4786_384f_25e3,
        0x0fc1_9dc6_8b8c_d5b5,
        0x240c_a1cc_77ac_9c65,
        0x2de9_2c6f_592b_0275,
        0x4a74_84aa_6ea6_e483,
        0x5cb0_a9dc_bd41_fbd4,
        0x76f9_88da_8311_53b5,
        0x983e_5152_ee66_dfab,
        0xa831_c66d_2db4_3210,
        0xb003_27c8_98fb_213f,
        0xbf59_7fc7_beef_0ee4,
        0xc6e0_0bf3_3da8_8fc2,
        0xd5a7_9147_930a_a725,
        0x06ca_6351_e003_826f,
        0x1429_2967_0a0e_6e70,
        0x27b7_0a85_46d2_2ffc,
        0x2e1b_2138_5c26_c926,
        0x4d2c_6dfc_5ac4_2aed,
        0x5338_0d13_9d95_b3df,
        0x650a_7354_8baf_63de,
        0x766a_0abb_3c77_b2a8,
        0x81c2_c92e_47ed_aee6,
        0x9272_2c85_1482_353b,
        0xa2bf_e8a1_4cf1_0364,
        0xa81a_664b_bc42_3001,
        0xc24b_8b70_d0f8_9791,
        0xc76c_51a3_0654_be30,
        0xd192_e819_d6ef_5218,
        0xd699_0624_5565_a910,
        0xf40e_3585_5771_202a,
        0x106a_a070_32bb_d1b8,
        0x19a4_c116_b8d2_d0c8,
        0x1e37_6c08_5141_ab53,
        0x2748_774c_df8e_eb99,
        0x34b0_bcb5_e19b_48a8,
        0x391c_0cb3_c5c9_5a63,
        0x4ed8_aa4a_e341_8acb,
        0x5b9c_ca4f_7763_e373,
        0x682e_6ff3_d6b2_b8a3,
        0x748f_82ee_5def_b2fc,
        0x78a5_636f_4317_2f60,
        0x84c8_7814_a1f0_ab72,
        0x8cc7_0208_1a64_39ec,
        0x90be_fffa_2363_1e28,
        0xa450_6ceb_de82_bde9,
        0xbef9_a3f7_b2c6_7915,
        0xc671_78f2_e372_532b,
        0xca27_3ece_ea26_619c,
        0xd186_b8c7_21c0_c207,
        0xeada_7dd6_cde0_eb1e,
        0xf57d_4f7f_ee6e_d178,
        0x06f0_67aa_7217_6fba,
        0x0a63_7dc5_a2c8_98a6,
        0x113f_9804_bef9_0dae,
        0x1b71_0b35_131c_471b,
        0x28db_77f5_2304_7d84,
        0x32ca_ab7b_40c7_2493,
        0x3c9e_be0a_15c9_bebc,
        0x431d_67c4_9c10_0d4c,
        0x4cc5_d4be_cb3e_42b6,
        0x597f_299c_fc65_7e2a,
        0x5fcb_6fab_3ad6_faec,
        0x6c44_198c_4a47_5817,
    ];
    let mut state: [u64; 8] = [
        0xcbbb_9d5d_c105_9ed8,
        0x629a_292a_367c_d507,
        0x9159_015a_3070_dd17,
        0x152f_ecd8_f70e_5939,
        0x6733_2667_ffc0_0b31,
        0x8eb4_4a87_6858_1511,
        0xdb0c_2e0d_64f9_8fa7,
        0x47b5_481d_befa_4fa4,
    ];
    let length = u128::try_from(bytes.len()).expect("a length fits in 128 bits") * 8;
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 128 != 112 {
        message.push(0);
    }
    message.extend(length.to_be_bytes());
    let (blocks, rest) = message.as_chunks::<128>();
    assert!(rest.is_empty(), "the padded message is whole blocks");
    for block in blocks {
        let mut w = [0_u64; 80];
        for (word, chunk) in w.iter_mut().zip(block.as_chunks::<8>().0) {
            *word = u64::from_be_bytes(*chunk);
        }
        for t in 16..80 {
            let s0 = w[t - 15].rotate_right(1) ^ w[t - 15].rotate_right(8) ^ (w[t - 15] >> 7);
            let s1 = w[t - 2].rotate_right(19) ^ w[t - 2].rotate_right(61) ^ (w[t - 2] >> 6);
            w[t] = w[t - 16]
                .wrapping_add(s0)
                .wrapping_add(w[t - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for (k, word) in K.iter().zip(w) {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let choice = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(*k)
                .wrapping_add(word);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (word, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *word = word.wrapping_add(add);
        }
    }
    let mut digest = [0_u8; 48];
    for (out, word) in digest.as_chunks_mut::<8>().0.iter_mut().zip(state) {
        *out = word.to_be_bytes();
    }
    digest
}
