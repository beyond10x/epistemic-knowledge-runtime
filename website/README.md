# Public project site

`index.html` and `styles.css` are the authored, script-free introduction to EKR.
Detailed guides remain under `docs/`. Change the product documentation there when
behavior changes, and keep this shorter entry point consistent with it.

`task site-check` validates the document, local anchors and project asset routes.
`task site-build` emits `website/build/`, including `.well-known/b10x-site.json`
bound to the current full Git revision. The builder is the Rust clap binary
`xtask/src/bin/ekr-docs.rs`; `task check` includes its tests and document validation.

The credential-free `pages.yml` workflow builds the exact artifact. The
repository-owned `b10x-docs-site.yml` calls the pinned shared project-site publisher
after a successful bot main push. It deploys `/epistemic-knowledge-runtime/` and
adds `.well-known/b10x-docs.json`. Before reporting publication, verify both live
provenance documents and the deployed HTML/CSS against the source commit.
