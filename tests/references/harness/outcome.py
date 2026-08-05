import subprocess
from dataclasses import dataclass
from enum import Enum


class Status(Enum):
    ACCEPTED = "accepted"
    REJECTED = "rejected"
    INVALID = "invalid"
    ERROR = "error"


@dataclass(frozen=True)
class Outcome:
    status: Status
    code: str | None = None
    entropy: str | None = None
    mnemonic: str | None = None
    details: tuple[str, ...] = ()

    @classmethod
    def parse(cls, record: str) -> "Outcome":
        fields = record.rstrip("\n").split("\t")
        try:
            status = Status(fields[0])
        except (IndexError, ValueError) as error:
            raise ValueError(f"invalid core record: {record!r}") from error
        if status is Status.ACCEPTED:
            if len(fields) != 3:
                raise ValueError(f"invalid accepted record: {record!r}")
            return cls(status=status, entropy=fields[1], mnemonic=fields[2])
        if len(fields) < 2:
            raise ValueError(f"invalid outcome record: {record!r}")
        return cls(status=status, code=fields[1], details=tuple(fields[2:]))


class CoreDriver:
    def __init__(self, executable: str) -> None:
        self._executable = executable

    def calculate(self, protocol: str, words: int, observations: str) -> Outcome:
        record = subprocess.check_output(
            [self._executable, protocol, str(words), observations], text=True
        )
        return Outcome.parse(record)


def require_accepted(label: str, outcome: Outcome) -> Outcome:
    if outcome.status is not Status.ACCEPTED:
        raise AssertionError(f"{label}: expected acceptance, received {outcome}")
    return outcome


def require_status(
    label: str,
    outcome: Outcome,
    status: Status,
    code: str,
    details: tuple[str, ...] = (),
) -> None:
    expected = Outcome(status=status, code=code, details=details)
    if outcome != expected:
        raise AssertionError(f"{label}: expected {expected}, received {outcome}")


def require_equal(label: str, expected: str, actual: str | None) -> None:
    if actual != expected:
        raise AssertionError(
            f"{label}:\nexpected: {expected}\nactual:   {actual}"
        )
