# Windows configuration write failure

The main session caused the Codex configuration parse failure while adding
the `azeroth-local` entry. It read a CRLF file as bytes, decoded those bytes,
then wrote the resulting string through Python's default Windows text mode.
That mode translated each existing LF again, producing CRCRLF. The pre-write
TOML check parsed the in-memory string, so it did not check the bytes that
Windows actually wrote.

A minimal isolated replay used only the unchanged model setting. The original
CRLF bytes passed native `codex features list`. The text-mode replay produced
CRCRLF and failed at exactly line 1, column 23: a carriage return must be followed
by a newline. No model setting change was involved.

Claude repaired the shared file. The later read-only audit found plain LF
endings, valid TOML and only the intended new MCP table as a semantic difference
from the saved backup. The main session left that repair unchanged. Reproduction
files are ignored local evidence and contain no credentials.

Future writes must normalise line endings, write explicit UTF-8 bytes, compare
the actual readback with the candidate, and run the native Codex parser. An
in-memory parser alone is not a sufficient check of a configuration write.
