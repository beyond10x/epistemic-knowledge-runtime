//! `task:lineage-shows-widened-ends-and-modified-properties`: each version of
//! `ekr.views.OverviewSchema` lists the edge-type ends it widened and the property declarations it
//! modified against its parent (`systems/ekr/domains/views.yaml`, `ekr.views.OverviewSchemaVersion`),
//! on both providers; and a store with no such version answers the bytes it answered before the
//! two fields existed.

mod support;

use ekr_core::RevisionNumber;
use ekr_views::{Index, OverviewRequest};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{self, Fixture, Provider};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// The overview of `fixture` at every revision, on `provider`, at the largest limit.
fn overviews(fixture: Fixture, provider: Provider) -> Vec<(Vec<u8>, ekr_views::GraphOverviewed)> {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    fixture.build(&runtime);
    let head = runtime.head().unwrap().unwrap().revision.get();
    (0..=head)
        .map(|at| {
            let index = Index::load(&runtime, Some(RevisionNumber::new(at))).unwrap();
            let answer = index
                .overview(&OverviewRequest::new(Some(OverviewRequest::MAX_LIMIT)).unwrap())
                .unwrap();
            (answer.bytes, answer.summary)
        })
        .collect()
}

/// `schema-changes` (`support::fixtures::build_schema_changes`): version 2 widens `observes`
/// (0x105), its source gaining `Subject` (0x100) and its target `Observation` (0x104); version 3
/// renames `label` (0x101) on `Subject` to `title` and makes it `Many`; version 4 declares `note`
/// (0x106) on `Observation` as well as on `Subject`. None of the three adds or removes an id.
#[test]
fn each_version_lists_the_ends_it_widened_and_the_properties_it_modified_on_both_providers() {
    let mut rendered = Vec::new();
    for provider in [Provider::File, Provider::Sqlite] {
        let answers = overviews(Fixture::SchemaChanges, provider);
        assert_eq!(
            answers.len(),
            5,
            "{}: the seed and four versions",
            provider.name()
        );
        let (bytes, summary) = answers.last().unwrap();
        let value: Value = serde_json::from_slice(bytes).unwrap();
        let versions = value["schema"]["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 5);
        for version in &versions[..2] {
            assert!(
                version.get("widened").is_none() && version.get("modified").is_none(),
                "{}: a version that only adds lists neither: {version}",
                provider.name()
            );
        }
        assert_eq!(
            versions[2],
            json!({
                "number": 2, "id": uuid(0x10c), "parent": uuid(0x107), "revision": 2,
                "added": [], "removed": [],
                "widened": [
                    {"edge_type": uuid(0x105), "side": "Source", "node_types": [uuid(0x100)]},
                    {"edge_type": uuid(0x105), "side": "Target", "node_types": [uuid(0x104)]},
                ],
            }),
            "{}",
            provider.name()
        );
        assert_eq!(
            versions[3],
            json!({
                "number": 3, "id": uuid(0x10d), "parent": uuid(0x10c), "revision": 3,
                "added": [], "removed": [],
                "modified": [{
                    "owner": uuid(0x100), "property": uuid(0x101), "name": "title",
                    "changed": ["Name", "Cardinality"],
                }],
            }),
            "{}",
            provider.name()
        );
        assert_eq!(
            versions[4],
            json!({
                "number": 4, "id": uuid(0x10e), "parent": uuid(0x10d), "revision": 4,
                "added": [], "removed": [],
                "modified": [{
                    "owner": uuid(0x104), "property": uuid(0x106), "name": "note",
                    "changed": ["Declared"],
                }],
            }),
            "{}",
            provider.name()
        );
        // Keys in the declared order: `widened` and `modified` follow `removed`.
        let text = std::str::from_utf8(bytes).unwrap();
        assert!(text.contains("\"removed\":[],\"widened\":[{\"edge_type\""));
        assert!(text.contains("\"removed\":[],\"modified\":[{\"owner\""));
        assert_eq!(
            (
                summary.added,
                summary.removed,
                summary.widened,
                summary.modified
            ),
            (5, 0, 2, 2),
            "{}",
            provider.name()
        );
        // Each version's changes appear from its own revision on, and not before.
        let counts: Vec<(u64, u64)> = answers
            .iter()
            .map(|(_, summary)| (summary.widened, summary.modified))
            .collect();
        assert_eq!(counts, [(0, 0), (0, 0), (2, 0), (2, 1), (2, 2)]);
        rendered.push(
            answers
                .into_iter()
                .map(|(bytes, _)| bytes)
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(
        rendered[0], rendered[1],
        "both providers answer the same bytes"
    );
}

/// The SHA-256 of every overview of `fixture`, revision 0 to the head, each at the largest limit,
/// as `<revision> <hex>` lines, hashed once more.
fn digest(answers: &[(Vec<u8>, ekr_views::GraphOverviewed)]) -> String {
    let mut lines = String::new();
    for (at, (bytes, _)) in answers.iter().enumerate() {
        lines.push_str(&format!("{at} {}\n", hex::encode(Sha256::digest(bytes))));
    }
    hex::encode(Sha256::digest(lines.as_bytes()))
}

/// Every other conformance fixture store, at every revision, answers exactly the bytes it
/// answered before `widened` and `modified` existed: the digests below were read off the base
/// (`wave/reads-04` 197d93d9) with this function, before the change.
#[test]
fn a_store_without_widened_ends_or_modified_properties_answers_the_bytes_it_answered_before() {
    const BEFORE: [(&str, &str); 12] = [
        (
            "seed-only",
            "3b8c0f26e9ed0fcd849ea1bc28d6d71c785d7e1862e48718c972bfdd3bcf6ee4",
        ),
        (
            "seeded-evidence",
            "2f2686be5c76f8d007ca740a1ef7f2f0623b1ae402ceffc96b83be7fa79b2e97",
        ),
        (
            "edge-assertion",
            "028dab297500ae34248526a2b9fb4400f1afb0e64884fde3eef947ad4734d68b",
        ),
        (
            "retracted-assertion",
            "0af6ec3b543fdcc51916055e60d3ea2b186d48cdd62e2d13e64a3fd426af76bd",
        ),
        (
            "schema-evolution",
            "1d408a3c1bd38979bf976434aa4bab7433cc1a9117759f64f4402429bef41e91",
        ),
        (
            "store",
            "8a6557427dc9a847a2ab69e4cfe72e199b17609c784c8911f3a10be8d01fff75",
        ),
        (
            "hub",
            "0e712e0a891cbd76e41bc7fc91b12dbacc3ccf4027d4bd11c939f4fe0b620156",
        ),
        (
            "timeline",
            "c70550cbf68ff5cd47d57443519b232632ac1af08b3a307511e59510aac6ff5a",
        ),
        (
            "growth",
            "b117d33d17b4da4102730865af1e79dfc5b14de0f0d507f90daac97c1d3473fe",
        ),
        (
            "subjects",
            "0e1906f915da671fe6da1e8576586ef507e0aa7e8ef2d57be86eee8118beb073",
        ),
        (
            "changes",
            "7a657ff66c41b9b505ad72990f2f9a4e3be3ea6224dde5363eb576b5013bd788",
        ),
        (
            "quality",
            "7843fec058a6f91ec20d81cec18a893ef2d157e0d50b394b526772eb00da7821",
        ),
    ];
    let mut differ = Vec::new();
    for (name, before) in BEFORE {
        let fixture = Fixture::named(name).unwrap();
        let answers = overviews(fixture, Provider::File);
        for (bytes, summary) in &answers {
            let text = std::str::from_utf8(bytes).unwrap();
            assert!(
                !text.contains("\"widened\"") && !text.contains("\"modified\""),
                "{name}: {text}"
            );
            assert_eq!((summary.widened, summary.modified), (0, 0), "{name}");
        }
        let now = digest(&answers);
        if now != before {
            differ.push(format!("(\"{name}\", \"{now}\"),"));
        }
    }
    assert!(
        differ.is_empty(),
        "differ from before:\n{}",
        differ.join("\n")
    );
}
