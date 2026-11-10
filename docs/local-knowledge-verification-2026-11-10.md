# Local hybrid knowledge verification

The public index contains 7,271 records from the pinned catalogue and
384-dimensional vectors from the checksum-pinned MiniLM model.
The local environment uses requirements-knowledge.txt. Model and index
files remain in ignored local-data directories.

The real CLI build completed, and a repeat returned unchanged without
changing index bytes. Two scoped tests passed in 21.877 seconds using
`python -m unittest test_forever_knowledge` with `PYTHONPATH=.;tests`.
They exercise the real local model through stdio MCP, block external
socket connections, assert source identities and hashes, reject invalid
inputs and verify unchanged source-index bytes.

Independent failure and security reviews found and corrected derived-index
reuse checks and unlisted optional model assets. Further failure rounds
extended the checksum to all FTS shadow tables and schema, including FTS
content row identities. The final failure round rejected eleven damaged
in-memory index cases and reached rebuild, while healthy CLI and real MCP
retrieval passed. Security checks rejected extra tokenizer assets before
reading them, detected changed schema/data, and exercised the actual SQLite
progress deadline. No reviewer modified the source or live index.

The security seal detects derived-file corruption. Exact public source
records remain checked independently against the pinned catalogue.
The companion has not been registered in existing client configuration or
deployed to the hosted gateway. Native game testing remains deferred.
Retrieval quality across general gameplay questions remains unbenchmarked.

Advisory judgements used Jev 1.14.0.

- Initial finding triage selected fix_both. Request
  `a5613d8422e3088f605aac46dd18c95249975aab9d4185b77ba8845d6d66883a`.
  Usage was 555 input and 58 output tokens in 0.311 seconds.
- Shadow-index triage selected fix. Request
  `5abcfd12721e7c3b66d8a1c7cc2caa8af2e48bdb78fcf761867a672186fd7928`.
  Usage was 468 input and 46 output tokens in 0.288 seconds.
- The scoped shipping judgement selected supported. Request
  `2c8fe39a92abf9be5824158c9cceb06b6beb933e4366c80f6815943a1ef12565`.
  Usage was 671 input and 45 output tokens in 0.302 seconds.

The author accepted the scoped offline result after the independent reviews.
