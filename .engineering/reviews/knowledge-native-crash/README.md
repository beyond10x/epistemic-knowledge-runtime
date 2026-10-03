# Native publication process-crash verification

The test starts a child over a real file or SQLite store with the production kernel authority and exact signed upgrade. A test-only port hook suspends it after durable election, either immediately before the native resume or after that resume succeeds. The parent waits for an explicit flushed readiness marker, verifies that its child is alive, kills that exact process and asserts Unix signal 9. A guard reaps the child even on assertion failure. No other process is touched.

The 4 observed combinations are recorded in measurements.json and process-kill.log. Before retry, full replay observes the seed revision for the pre-publication kill and the upgraded revision for the post-publication kill. A separate fresh retry process must reuse the elected receipt byte for byte without sampling the clock. Another reopen verifies the original historical graph, disputed claims, the outstanding question, and an idempotent repeat with no new revision.

These are actual process deaths at EKR native-publication boundaries. They do not inject a kill inside filesystem sync or SQLite WAL execution, simulate power loss, or establish storage-hardware durability. Existing port uncertainty and native concurrency tests remain complementary. The test is Unix-only because it asserts the termination signal. This is root-local verification, not independent review or the full PR gate.

Focused kernel test-target Clippy passed with warnings denied. Authority-upgrade and shared recovery regression outputs: [{"passed":6,"failed":0,"ignored":0},{"passed":5,"failed":0,"ignored":0}]. Exact logs are retained alongside this record.
