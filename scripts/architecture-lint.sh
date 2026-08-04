#!/usr/bin/env bash
set -euo pipefail

fail=0

if rg -n 'termion|crate::ui|super::ui' src/presentation; then
  echo 'presentation must remain independent of terminal adapters' >&2
  fail=1
fi

if rg -n 'ceremony\.handle|Ceremony::handle|\b(generate|calculate)\(' src/ui; then
  echo 'UI must call application sessions instead of orchestrating the domain' >&2
  fail=1
fi

if rg -n 'Command::(ConfirmRolls|RecordGenerationSucceeded|RecordExactAttemptRejected|VerifyMnemonicBackup)' src/ui; then
  echo 'UI must not issue application-owned aggregate facts directly' >&2
  fail=1
fi

if rg -n 'BitcoinSha256|Bip39Encoder' src/ui/app.rs src/ui/render; then
  echo 'production adapters belong only in the terminal composition root' >&2
  fail=1
fi

if rg -n '^\s*pub fn events\(' src/domain/ceremony/entity.rs; then
  echo 'raw secret ceremony events must remain application-internal' >&2
  fail=1
fi

if rg -n 'termion|crate::(adapters|application|presentation|ui)|bip39_ceremony_tui::' \
  crates/bip39-ceremony-core/src; then
  echo 'calculation core must not depend on application or terminal layers' >&2
  fail=1
fi

if rg -n 'use (bip39|bitcoin_hashes)(::|[[:space:]]|\{)|extern crate (bip39|bitcoin_hashes);' src \
  --glob '!**/adapters/crypto.rs'; then
  echo 'protocol cryptography belongs in the calculation core' >&2
  fail=1
fi

if (( fail != 0 )); then
  exit 1
fi

echo 'architecture lint passed'
