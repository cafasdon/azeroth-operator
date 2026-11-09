# Real-engine saved empty-ring workflow — 9 November 2026

## Scope

Verify the actual local MCP-to-simulator caller path after empty-slot search
integration. This test changes no runtime source, addon, deployment or game
state. The character capture is synthetic, uses public catalogue item IDs, and
is held only in a temporary owner store.

This is workflow and deterministic request/checkpoint evidence for one public
Hunter rotation and one encounter. It does not validate the model against the
game, native legality, complete inventory, full-set options, all classes/roles,
multiple encounter mechanics, statistical optimality or full owned BiS.

## Fixed setup

- Positional engine SHA-256:
  `833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.
- Public Hunter template SHA-256:
  `761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0`.
  Template rotation, buffs and settings are public upstream data; captured
  equipment replaces the template's equipment through the normal adapter.
- Synthetic Forever level-60 Orc Hunter; Engineering/Blacksmithing 300;
  observed active pet level 60. Equipped ring IDs 4549/4550, main-hand 2042 and
  ranged 22347. No enchants or suffixes; empty observed bags; bank coverage
  remains unavailable. Search only finger1/finger2, with other slots preserved.
- One 45-second encounter, seed 20261109, 200 paired iterations per request.
  Three collection windows cover nine coordinates and seven distinct resolved
  equipment states. Positional capability verification and simulation calls
  run the real local executable. No simulator or MCP mocks; only the owner-store
  environment is scoped with patch.dict.

## Author invocation and result

With `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`, explicit
`AZEROTH_POSITIONAL_ENGINE` and `AZEROTH_QUALIFY_TEMPLATE` paths:

`python -m unittest test_real_empty_gear_workflow -v`

Initial run: one test, 15.646 seconds, OK. Seven cold engine calls (1,400
iterations), zero new warm calls. Seven additional uncached engine runs use
fresh requests reconstructed through the actual MCP preparer. Every final mean,
standard deviation, baseline-paired standard error, approximate interval and
request hash agrees exactly with the direct reference at the fixed seed.
The reference uses the same engine and model, not an independent model.

Mean DPS in ranked order:

655.1477812701295, 655.1477812701295, 654.5399932368693,
654.5399932368693, 653.2083183730423, 653.2083183730423,
652.6005092110964.

Ties are retained. These point estimates are not a claim that one gear state
is statistically better. The all-empty ring alternative remains among retained
results. Explicit clears survive the full saved workflow. Journal and snapshot
bytes remain unchanged during warm execution and direct reference checks.
No private character, host/account identity, raw iteration samples or temporary
paths are printed. Temporary journals are closed and removed by test cleanup.

## Plan advisory

Jev 1.14.0 request
`bcadc3b74f409d76a90bf7efe8e9b7c2e2ba2fe41d16743e3a2cccf8a71bae6e`:
proceed .53 / revise .27 / insufficient .20; 562 input, 47 output, .264s.
This low-confidence advisory was not treated as proof. The author proceeded
with exact hashes, explicit opt-in inputs, real CLI calls and uncached numerical
cross-checks to resolve the practical verification question directly.

## Independent review and final disposition

One clean gpt-6-astra high Failure/claim-scope lens ran the actual opt-in test:
one test, 17.264s, OK. It traced real CLI execution, scope and all assertions;
no concrete fault found. Callers/security/deployment lenses omitted for this
test/prose-only unit with no runtime guard or live change. Agent token usage
not exposed. Final author run after adding a sorted point-estimate assertion:
one test, 15.409s, OK, identical numeric output.

The paired reference uses the same adapter comparison routine; it verifies
saved-workflow fidelity, not an independent proof of the uncertainty formula.

Final Jev 1.14.0 advisory request
`43f0be307444b4863b253285325fbba40104ae0b3aab33932658861997a25fbc`:
land .88 / fix .02 / insufficient .10; 636 input, 47 output, .327s.
Author disposition: land the bounded qualification on actual caller evidence;
keep the broader feature/native/model requirements and overall goal open.
