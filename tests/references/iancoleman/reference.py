import json
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class IanOutcome:
    entropy: str
    mnemonic: str


class IanColeman:
    def __init__(self, node: str, runner: Path, source: str) -> None:
        self._node = node
        self._runner = runner
        self._source = source

    def from_entropy(self, entropy: str) -> IanOutcome:
        return self._run("entropy", entropy)

    def _run(self, operation: str, value: str) -> IanOutcome:
        output = subprocess.check_output(
            [self._node, str(self._runner), self._source, operation, value],
            text=True,
        )
        decoded = json.loads(output)
        return IanOutcome(
            entropy=decoded["entropy"], mnemonic=decoded["mnemonic"]
        )
