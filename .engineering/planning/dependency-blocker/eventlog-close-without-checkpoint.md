---
format: aep.planning-md/3
id: dependency-blocker:eventlog-close-without-checkpoint
kind: dependency-blocker
status: open
title: Replacement-safe close needs an upstream Eventlog capability
relations:
- blocks: task:replaced-store-close-keeps-the-restored-file
revision: 1
---
The task's Build section requires an Eventlog SQLite connection-close operation that does not
checkpoint its old WAL into a restored file. The currently pinned provider does not expose that
capability, as scoped by the correctness review. Clear with a governed upstream release and
approved EKR pin update whose replacement and ordinary-close tests execute. The next EKR-local
waves do not authorize an upstream release.
