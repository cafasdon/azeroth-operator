# Exact incoming public references

The public companion's existing exact get tool now accepts
`include_incoming_references=true`. This answers which cached records point to a
selected NPC, zone, faction, random suffix or spell effect through recognised
fields. It does not infer which items are currently obtainable or owned.

The returned record gains `incoming_catalogue_references`. Each edge carries an
exact source key, JSON pointer, source section, source-row SHA-256 and target key.
Suffix options retain their catalogue-only relationship. Duplicate references at
different pointers remain separate edges. Names and similarity scores never join
records. Unknown root keys retain `found=false` without a reverse report.

Pages contain up to 32 edges in sorted source-key and stable source-field order.
`incoming_reference_offset` is uint32 and requires the incoming option when
nonzero. The count covers all recognised matching edges in this pinned catalogue;
it is not a complete world or acquisition count. Advance the offset by 32 while
`has_more_references` is true. An offset beyond the end returns an empty page.

The raw recognised-field extractor is shared with forward references. Validation
still checks the existing public index once per lookup. Reverse scans use no
embedding calls and have a five-second analysis deadline. A deadline error gives
no partial count. The existing 256 KiB response bound still applies. No persistent
reverse index or format migration is needed for the bounded pinned corpus.

CLI: `get --include-incoming-references --incoming-reference-offset 0`, with the
usual explicit catalogue/index/model and record-key options. The CLI rejects
incoming flags for other actions before model construction. Forward/deep views
can coexist with this independently paged reverse view. Omitted options preserve
the old response and work. Tool count remains two. Search is unchanged.

No private capture, game action, network lookup, installed profile or native
qualification is added. Owner-private data never enters this public source map.
