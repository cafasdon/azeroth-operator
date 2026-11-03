"""Explicit client boundaries. Unknown and unqualified clients fail closed."""
from dataclasses import dataclass
from enum import StrEnum


class ClientFamily(StrEnum):
    RETAIL = "retail"
    CLASSIC = "classic"
    FOREVER = "forever"


@dataclass(frozen=True)
class ClientIdentity:
    family: ClientFamily
    build: str
    interface: int
    variant: str

    def __post_init__(self):
        if not isinstance(self.family, ClientFamily):
            raise ValueError("Explicit ClientFamily required")
        if not self.build.strip() or not self.variant.strip():
            raise ValueError("Build and variant must be explicit")
        if type(self.interface) is not int or self.interface <= 0:
            raise ValueError("Positive interface version required")

    @property
    def key(self) -> tuple[str, str, str, int]:
        return (self.family.value, self.variant, self.build, self.interface)


def require_same_client(source: ClientIdentity, target: ClientIdentity) -> None:
    """Prevent accidental cross-family, cross-variant or cross-build imports."""
    if source.key != target.key:
        raise ValueError("Client mismatch: explicit conversion and qualification required")
