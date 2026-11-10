# Saved map target level verification

The offline locked build and two tests in `test_saved_reset_projection` passed.
An independent failure review ran 28 real stdio MCP calls covering endpoint
levels, inclusive bounds, missing data, invalid inputs, filter composition,
default behaviour and unchanged snapshot bytes. It found no defect.

The advisory shipping judgement selected supported using Jev 1.14.0
(`jev-latest`). Request SHA-256
`3e96bb444df9e1ec13fbee7936e557c8b0adcd774911ebdb4cfd2b9189abd563`.
Usage was 497 input and 45 output tokens in 0.343 seconds.
The independent reviewer and author accepted the scoped offline change.
Native game verification remains deferred.
