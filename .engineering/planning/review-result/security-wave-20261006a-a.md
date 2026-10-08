---
format: aep.planning-md/3
id: review-result:security-wave-20261006a-a
kind: review-result
status: active
title: Security review, wave 20261006a unit A (password file)
relations:
- reviews: story:postgres-password-file
revision: 1
---
unit: story:postgres-password-file (unit A, EKR wave 20261006a), uncommitted working tree at `~/.local/state/worktree/trees/b10x/epistemic-knowledge-runtime/ekr-w6a-a` on base `4a26b955`
verdict: INFEASIBLE (one note); all six invariants hold
cases: executed 190→198, red 1
origin: introduced 0 / pre-existing 0 / undecided 1
write-outside-worktree: 2 paths
needs-coordinator: none

**1. `git --no-pager diff --stat`**
```
 CHANGELOG.md                               |  9 ++++
 Cargo.lock                                 |  1 +
 Cargo.toml                                 |  1 +
 crates/ekr-store/Cargo.toml                |  1 +
 crates/ekr-store/src/postgres.rs           | 71 ++++++++++++++++++++++++++++--
 crates/ekr-store/tests/runtime_context.rs  |  1 +
 docs/cli.md                                | 23 ++++++++++
 docs/epistemic-knowledge-runtime-design.md |  4 ++
```
These are all the implementor's changes, and the stat is the same as before I started. My only change is one new untracked test file, `crates/ekr-store/tests/postgres_password_file_review.rs`. I edited no non-test code.

**2. Cases added** (in the file above; run on their own against the docker fixture, log at `~/.cache/ekr-wave-20261006a/a/review-cases.log`)

| Case | What it checks | Now |
|---|---|---|
| `adversarial_passwords_reach_the_next_stage_in_both_forms_unechoed` | 14 passwords (quotes, backslashes, trailing `\`, spaces, `=`, `&`, `#`, `%`, `%00`, CR/LF/tab, NUL, Unicode, embedded `host=`, `sslmode=disable`, `password=`) × 4 connection forms all pass the reparse check; no sentinel in Display, Debug or config Debug | green |
| `a_failed_reparse_is_refused_with_a_fixed_category_and_unechoed` | connection files that parse alone but break once the password is added (trailing `\`, `?` in URL userinfo, URL ending in `?`) give the fixed "cannot be applied" error, unechoed | green |
| `every_form_of_a_connection_password_is_refused` | 9 forms with a password (key=value, spaced, empty `''`, multi-line, URL userinfo, empty userinfo password, query param, empty query param, `pass%77ord`) are refused as "carries a password"; `PGPASSWORD=`, `PASSWORD=`, `passfile=` and URL `?PGPASSWORD=` are refused as "invalid connection file" | green |
| `a_sealed_memfd_is_read_whole_on_every_construction` | `/proc/self/fd/N` of a sealed memfd passes the password stage 3 times | green |
| `failing_password_files_are_refused_with_fixed_categories` | exactly 65536 B accepted, 65537 B refused; absent, mode 000 and directory give "cannot read"; empty, invalid UTF-8, lone surrogate, BOM, array and nested object give "invalid password file"; each category's text is identical across its cases | green |
| `the_field_is_optional_and_the_document_stays_closed` | no field reads as `None`; the near-miss `passwordfile` is still refused | green |
| `adversarial_passwords_open_the_hosted_store_as_the_configured_role` | for 13 adversarial passwords, a dedicated role's password is set through `psql :'pw'`, then the store opens in key=value, URL and URL-with-query form; SCRAM proves the exact password arrived, and an injected `host`/`port`/`dbname` would have pointed the connection somewhere that doesn't exist | green |
| `a_password_file_without_a_writer_is_refused_rather_than_awaited` | a named FIFO with no writer is refused within 5 s | **red** |

The red case's output, verbatim:
```
thread 'postgres_password_file_review::a_password_file_without_a_writer_is_refused_rather_than_awaited' (1608667) panicked at crates/ekr-store/tests/postgres_password_file_review.rs:347:31:
still blocked opening the password file after 5 s: Timeout
```
On the first run the hosted case was also red, but the cause was my fixture: I made the review role a member of `ekr_app`, and eventlog's admission check refuses role membership. I gave the role direct grants instead and it went green. That was not a unit defect.

**3. Suite run**
`cargo test --locked -p ekr-store --no-fail-fast` with the brief's env plus `EKR_TEST_POSTGRES_CONFIG`, `EKR_TEST_POSTGRES_OWNER`, `EKR_REVIEW_POSTGRES_CONTAINER=ekr-w6a-a-postgres` and `EKR_REQUIRE_POSTGRES=1`.
- **Result:** 197 passed, 1 failed (the FIFO case above), 2 ignored; `EXIT=101`.
- **Implementor's hosted cases:** all 7 `postgres_password_file` cases passed against the fixture, including the memfd-opened-twice case.
- **"Before" count:** 190, taken from the implementor's `green-ekr-store.log`.
- **Log:** `~/.cache/ekr-wave-20261006a/a/review-suite.log`.
- **Cleanup:** I removed the container afterwards.

**4. Findings**

| file:line | What was measured | What reaches it | Verdict / origin |
|---|---|---|---|
| `crates/ekr-store/src/postgres.rs:88` | `bounded_file` calls `File::open` with no non-blocking flag and no file-type check, so a `password_file` naming a FIFO with no writer blocks forever instead of being refused. A FIFO whose writer delivers once will also block on the second read. | Nothing found: the documented handover is a sealed memfd, and only a misconfigured path reaches this. `connection_file` and `ca_file` go through the same function. | INFEASIBLE / undecided (not run against the base) |

Suggested fix (not applied): open with `O_NONBLOCK`, or refuse anything that is not a regular file or memfd. Then this case passes.

**5. Reviewed and could not fault**
- **Invariant 1:** every error is a fixed string, and parser errors are thrown away (`postgres.rs:113,121,140`). `PasswordDocument` has no `Debug`. `tokio_postgres::Config`'s `Debug` hides the password. eventlog's `PostgresConfig` has no `Debug`. All of this was checked across the success, refusal and reparse-failure paths.
- **Invariant 2:** the escaping matches tokio-postgres 0.7.18's quoted-value parser exactly, and the URL encoding leaves only unreserved characters. Even with a trailing-`\` connection file that escapes the quoting, a length argument shows the reparse check can never read back the full password, so that case is refused and nothing is injected. `sslmode` is forced to `Require` by `verified` anyway.
- **Invariant 3:** the check uses the same parser as `verified`, so any password it can see is caught. tokio-postgres reads neither the `PGPASSWORD` environment variable nor `.pgpass`.
- **Invariant 4:** memfd works twice, both without and with a database. If fd 3 were a pipe instead, it would yield the document once and the second read would be refused as "invalid password file". The contract names a memfd, so I don't raise this.
- **Invariants 5 and 6:** hold, apart from the FIFO note. A `"password_file": null` value is accepted and treated as absent.

**6. Paths written outside the worktree**
- `~/.cache/ekr-wave-20261006a/a/review-cases.log`
- `~/.cache/ekr-wave-20261006a/a/review-suite.log`

Test temp directories went under the brief's `TMPDIR` and were cleaned up. The cargo build output went to the brief's shared target directory. Container `ekr-w6a-a-postgres` is removed; the `ekr_review` role I created went with it.

**7. Findings block**
```findings
[
  {
    "file": "crates/ekr-store/src/postgres.rs",
    "line": 88,
    "category": "boundary",
    "severity": "note",
    "verdict": "INFEASIBLE",
    "origin": "undecided",
    "message": "a password_file naming a FIFO with no writer blocks forever in File::open instead of being refused with a fixed category; red case a_password_file_without_a_writer_is_refused_rather_than_awaited."
  }
]
```