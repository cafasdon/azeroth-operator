# Default reader gateway probe: offline evidence

The probe's nine-tool contract omitted resolve_character, search_forever_recipes
and plan_forever_recipe_restock, all present in the existing default profile.
This made current default-source discovery fail before later policy checks.

The probe now independently expects the twelve default reader tools. It rejects
missing/extra/duplicate/malformed entries and requires exact boolean read-only
and non-destructive annotations. Catalogue-only tools remain excluded from this
default profile contract. This changes no gateway grants, profile, server policy,
owner store, token or installed service. Existing current-inspection schema and
blocked preparation-route checks remain in place. Their runtime guards now use
explicit ValueError checks rather than removable assert statements. HTTPError responses close on
the error return path; the offline HTTP fixture exposed their previous leak.

Direct checks:

- `PYTHONPATH=.;tests python -B -m unittest test_gateway_probe test_gateway_profile test_gateway_upgrade`:
  initial 7 tests passed, 0.454 seconds.
- `python -B deploy/probe_gateway.py --help`: passed without token access.
- `python -W error::ResourceWarning -B -m unittest test_gateway_probe test_gateway_profile test_gateway_upgrade`
  with `PYTHONPATH=.;tests`: final 7 tests passed, 0.446 seconds.
- `git diff --check`: passed without output.

The new test obtains actual tool discovery, capabilities and empty snapshot
results from target/debug/azeroth-operator.exe in an isolated owner-local store.
It compares discovery to the rendered default profile and exercises drift,
duplicates, mutation/catalogue-only substitutions, missing annotations and
non-boolean hints. It also executes the real probe script with a controlled
offline HTTP boundary, checks all 14 requests and seven preparation denials,
asserts the token is absent from output, and proves the script rejects the old
nine-tool catalogue. HTTP authentication in that fixture is simulated; this is
not evidence of installed gateway authentication or native/live qualification.

Further offline author review reproduced an optimisation fault: compile level 1
removed the two auth requests inside assert expressions. The test observed only
12 HTTP requests instead of 14, while the script still printed auth success.
Every runtime assert in the probe now uses an explicit condition check, keeping
both requests and all policy/status/route/snapshot checks active at -O/-OO.

The script fixture now compiles and executes the actual source at optimisation
levels 0, 1 and 2. Each valid fixture makes all 14 requests. Each level rejects
accepted missing credentials and eleven additional faults: accepted wrong token,
wrong policy mode, enabled game imports, wrong snapshot scope, capability error,
snapshot-list error or wrong type, successful preparation, argument-error-only
preparation refusal, failed discovery and failed initialised notification. No
live HTTP server is contacted by this fixture.

- Final scoped regression with ResourceWarning treated as error: 7 passed,
  0.762 seconds (`test_gateway_probe test_gateway_profile test_gateway_upgrade`).
- `python -O -W error::ResourceWarning -B -m unittest test_gateway_probe`
  with `PYTHONPATH=.;tests`: 1 passed, 0.489 seconds.
- Final `git diff --check`: passed without output.

The initial review pause was an author error: the standing instructions already
authorise the required sub-agent reviews. Two independent lenses (callers and
failure/security), each requested on gpt-6-astra at high effort, completed with
no code findings. Both flagged the obsolete review/spending paragraph in this
record; it is corrected here. No source code changed after their probes.

Callers ran the seven scoped tests: 7 passed, 0.586 seconds. Its actual -OO test
passed in 0.525 seconds. Standalone --help passed without token access. An
independent fully mocked upgrade entrypoint confirmed success probes before
enablement (9 process calls) and probe failure stops/disables the reader
(8 calls). Nonempty HTTPError response bodies closed. Duplicate, malformed
and numeric-boolean substitutions were refused.

Failure/security ran the actual -O and -OO tests: 1 passed each, 0.336 and
0.333 seconds. Its separate complete-script harness ran 24 cases across compile
levels 0/1/2: valid responses, null/missing initialisation, null discovery/result/
tools, null tool entries and duplicate replacement. Failures produced no success
output; synthetic tokens appeared in neither stdout nor tracebacks; every
response body closed. No network, live service or private capture was accessed.

The final callers readback found no remaining actionable issue in the corrected
record. The failure lens did not repeat because only this factual status record
changed after the first round. Root judgement: ready for source integration.
No external Jev call, purchase, deployment or live probe occurred. This remains
offline source evidence, not installed gateway authentication or game acceptance.
