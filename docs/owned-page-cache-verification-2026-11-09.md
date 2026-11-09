# Owned-gear page reuse offline verification

In the synthetic 257-location, two-slot fixture, two windows made 24 owned-gear
page requests without reuse and 12 with reuse. Saved search scopes, exclusions
and resolved drafts matched. Fresh stats and equipment review calls still ran.
A one-byte cache fell back to 24 requests with identical journal progress.

`python -m unittest test_owned_page_cache` passed all three tests. The tightened
source-change test separately confirmed that modified captures and catalogues
stopped collection before a cached lookup or second append. The scoped real
module resume regression passed in 14.740 seconds.

ADR-074 records the invocation lifetime and the selected limits of 32 MiB of
serialized keys and payloads and 1,024 entries. These checks measured request
counts on the local MCP using synthetic data. Live testing remains deferred.

Jev 1.14.0 plan advisory `6f1d8244f31c2fe832b47bfe7b8094f29707f3d41cbacb1bc015eef7a70b9dc7`
returned proceed .75 at confidence .62, with 675 input and 47 output tokens in
.420 seconds. Integration advisory `10b0d7205b133bd2560e35e44a92c7181b200faded5e8aa8877a179c800dcd71`
returned integrate .76 at confidence .64, with 411 input and 49 output tokens in
.334 seconds. Direct offline tests and independent reviews govern integration.

Independent Callers and Failure/privacy reviews passed all three cache tests.
Additional probes covered stale catalogue rejection, replacement accounting,
uncacheable values and numeric-only statistics. Both reviews found no defect.
