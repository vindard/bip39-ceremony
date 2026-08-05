#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const [source, operation, input] = process.argv.slice(2);
if (!source || !operation || !input) {
  throw new Error("usage: iancoleman-reference SOURCE OPERATION INPUT");
}

global.window = global;
global.$ = () => ({ find: () => ({ val: () => 2048 }) });
for (const file of [
  "sjcl-bip39.js",
  "wordlist_english.js",
  "jsbip39.js",
  "entropy.js",
]) {
  const filename = path.join(source, "src/js", file);
  vm.runInThisContext(fs.readFileSync(filename, "utf8"), { filename });
}

const mnemonic = new Mnemonic("english");
let entropy;
if (operation === "entropy") {
  if (!/^(?:[0-9a-f]{32}|[0-9a-f]{64})$/.test(input)) {
    throw new Error("entropy must be canonical 128- or 256-bit lowercase hex");
  }
  entropy = input;
} else if (operation === "legacy-dice") {
  if (!/^[1-6]+$/.test(input)) {
    throw new Error("legacy dice input must contain only die faces");
  }
  const cleanInput = window.Entropy.fromString(input, "dice").cleanStr;
  entropy = sjcl.codec.hex.fromBits(sjcl.hash.sha256.hash(cleanInput));
} else {
  throw new Error(`unsupported operation: ${operation}`);
}

const bytes = [...Buffer.from(entropy, "hex")];
process.stdout.write(JSON.stringify({
  entropy,
  mnemonic: mnemonic.toMnemonic(bytes),
}));
