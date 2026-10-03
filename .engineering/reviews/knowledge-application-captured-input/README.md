# Captured inputs for application replay

The existing read, interpretation validation and proposal material paths now share pure helpers
that receive admitted replay state and captured inputs. This removes the need for future application
verification to reenter provider reads. It does not implement application authorization or publication.

This is a refactor, not a claimed new F behavior. No red mutation was executed for this increment.
The existing regression command was:
`cargo test --locked -p ekr-kernel --lib --test knowledge_retention --test schema_proposal_reviews -- --nocapture`.
Its own summaries report 31 library, 14 retention and 8 review cases passed, with no failures or
ignored cases. Strict kernel library clippy and exact changed-file rustfmt checks returned zero.
Independent source/log review found no dropped checks, material-byte change or snapshot/cache defect;
the reviewer performed no independent execution.

Logs replace local worktree/build paths and normalize trailing blank lines. Original complete logs
remain in the assigned private evidence directory. Actual process status files are copied unchanged.
The full F implementation, provider application tests and integrated task check remain outstanding.
