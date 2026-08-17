// Drives BlueWallet's own extracted functions the way its dice screen does:
// each face is a 0-indexed button press, pushed into the reducer, and the
// finished state is converted to bytes. Nothing here reimplements the encoding.

const path = require('node:path');

const [, , adapterPath, bignumberPath, words, rolls] = process.argv;

globalThis.__BN = require(path.resolve(bignumberPath)).BigNumber;

const { EActionType, eReducer, getEntropy, convertToBuffer } = require(path.resolve(adapterPath));

const limit = Number(words) === 24 ? 256 : 128;

let state = { entropy: globalThis.__BN(0), bits: 0, items: [], limit };
let consumed = 0;

for (const character of rolls) {
  const face = Number(character);
  // ProvideEntropy renders one button per face index 0..sides-1.
  const pushed = getEntropy(face - 1, 6);
  const next = eReducer(state, { type: EActionType.push, value: pushed.value, bits: pushed.bits });
  if (next.bits !== state.bits) {
    consumed += 1;
  }
  state = next;
}

const buffer = convertToBuffer({ entropy: state.entropy, bits: state.bits });
const hex = Buffer.from(buffer).toString('hex');

process.stdout.write(`${state.bits}\t${consumed}\t${hex}\n`);
