# Pilot repairs and installed verification — 14 November 2026

## Result

The repaired installed capture-to-upload-to-hosted-MCP path passed real
Forever character checks. All 20 captured known Leatherworking recipes now
appear in search, and each passes individual bulk material review. The default
Forever talent endpoint returns the captured allocation. An exact fresh
caller-supplied session reference works, with explicit-character precedence
and stale-reference rejection. Automatic live identity detection remains
unavailable; this last change is bounded session selection, not detection.

## Corrections

1. The client returned zero `GetProfessionSpells` IDs for an open local page,
   while `GetAllRecipeIDs` supplied 592 entries and explicit recipe metadata
   identified 20 as learned. The exporter now uses the guarded latter path
   only when the first list is empty. It preserves local/context/secret guards
   and never treats missing learning data as a known recipe.
2. The selected Camp Tent observation reported profession 2945, while the open
   local page and bulk capture reported 165. Its output and materials agreed,
   but the existing conflict guard correctly rejected the different IDs.
   The exporter now omits optional selected-recipe profession metadata unless
   its ID exactly matches the local page. It neither converts namespaces nor
   weakens the analysis guard. Recipe identity, output and reagents survive.
3. `captured_talent_loadout` no longer rejects all Forever data. It returns
   exact captured nodes and definition observations for review, without a
   native import string or a simulator compatibility claim.
4. `resolve_character.current_snapshot_id` accepts an exact owner-local export
   less than 15 minutes old. Explicit identity overrides it. Future, stale,
   missing, corrupt or mismatched references never select another capture.
   Both live identity flags remain false, and retries do not renew expiry.

## Installed evidence

- Native client: build 70938, interface 16001. Capture-only addon
  `0.21.2-pilot`, four verified files, archive SHA-256
  `b1b95284132e4648a1db7f627371853878d47556b7b8bf2f4c1de13b2019b9e9`.
  Exact previous files remain in local rollback backups. Reload, capture and
  subsequent persistence were observed separately in the native UI.
- The existing pinned Windows uploader imported and uploaded the new export.
  Its receipt reported `uploaded`; the unrelated Retail capture stayed unchanged.
- Personal pylon CT290 reader and importer use image
  `sha256:e12095d0a3413e593cd099a0c768ed823b9e5cbe6702723189ebf7e2fe77555e`,
  built from source revision `d8d4ba1`. Later addon-only changes do not alter
  this image's Rust/clients build inputs. Upgrade preconditions preserved the
  fixed owner store, read-only mount and existing authentication settings.
  Importer fixture checks and reader checks passed.
- Independent MCP SDK clients tested the real capture through local stdio and
  public authenticated HTTPS. Both searched 20 recipes and reviewed all 20
  material sets against the exact local imported data.
- The native talent UI showed 13 spent points across five active nodes. Both
  MCP clients returned exactly those captured ranks, with no generated import.
- Both clients selected the exact fresh session reference, honoured explicit
  identity over an invalid reference and rejected an older real capture.
- Public HTTPS returned 401 for missing and wrong tokens, discovered 12 tools
  with correct authentication and blocked all seven tested preparation routes.

Raw captures, character identifiers, GUIDs, wallet amounts, tokens and
screenshots remain in ignored local acceptance storage. The committed
[aggregate results](pilot-repair-results-2026-11-14.json) contain none of these.

## Tests and review

Direct tests covered profession spells, bulk materials and cooldowns, planning,
talent allocation/definitions, character resolution and addon round trips.
The fixed-clock session test covered exact expiry and future timestamps.
Real stdio tests covered owner isolation, filters, malformed data and explicit
precedence. Native regression checks preserved uncertain bank state and did
not claim craftability or complete ownership.

Each change received an independent scoped review. The talent review found
two stale test expectations (medium: the changed response broke real test
callers); both were fixed and their focused recheck passed. The other reviews
found no confirmed medium, high or low issue. Existing conflict guards stayed
intact. No CI or full-suite replay ran. Review agents used the requested
GPT-6 Astra/high tier. The active-identity investigation also recorded an
advisory Jev `jev-1.14.0` judgement: disk-only observations cannot prove current
play, request hash `1035eeb7362c6a64aee46c7cc0fa085993d25066208c60f9291057fb44ea9569`,
590 input / 54 output tokens, 0.31 seconds. It was not completion evidence.

## Remaining limits

The current game session supplied these captures, but saved data cannot prove
that a character remains in play after a switch or logout. The new reference
does not solve automatic detection. Full gear optimisation, simulator output,
native talent import, complete bank/mail coverage and player-resource capture
also remain separate unqualified paths. No gameplay action, purchase, craft,
quest change, trade, social message or inventory mutation was needed here.
