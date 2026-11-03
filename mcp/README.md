# MCP interface

Shared transport; every character and proposal carries family, variant, build and interface version. No implicit default client. Unsupported operations return unavailable rather than falling back to another client adapter.

Planned initial operations: list snapshots, inspect character, compare snapshots, analyse equipment, prepare player-reviewed configuration proposals. No game execution endpoint.
