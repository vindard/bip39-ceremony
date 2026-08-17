#!/usr/bin/env node
// Runs iancoleman's own `setMnemonicFromEntropy` over a dice tape.
//
// The function is DOM-bound, so everything it reads is stubbed and everything
// it writes is captured. The two inputs that decide the construction are the
// entropy string and the mnemonic-length setting; the phrase it produces is
// read back out. No part of the conversion is reimplemented here.

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const [source, adapter, lengthSetting, rolls] = process.argv.slice(2);
if (!source || !adapter || !lengthSetting || !rolls) {
  throw new Error("usage: dice-runner SOURCE ADAPTER LENGTH ROLLS");
}

global.window = global;
// jsbip39 touches jQuery at construction; the existing bip39 runner stubs it the same way.
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

// zxcvbn only drives the crack-time readout; a stub keeps feedback harmless.
// `bip39-libs.js` is a browserify bundle that will not load outside a browser,
// so the two utilities the extracted function reaches for are supplied here.
// Neither decides the construction: zxcvbn only drives the crack-time readout,
// and BigInteger is used solely to render a hex digest as a binary string.
// Which string gets hashed, the truncation to numberOfBits, the floor to whole
// 32-bit groups and the trailing-bit selection all remain upstream's code.
global.libs = {
  zxcvbn: () => ({ crack_times_display: {}, feedback: { warning: "" } }),
  BigInteger: {
    BigInteger: {
      parse: (value, radix) => {
        if (radix !== 16) {
          throw new Error(`unexpected radix: ${radix}`);
        }
        const parsed = BigInt(`0x${value}`);
        return { toString: (outRadix) => parsed.toString(outRadix) };
      },
    },
  },
};

const mnemonic = new Mnemonic("english");
let phrase = null;

const noopNode = new Proxy(
  {},
  {
    get: () => (...args) => (args.length ? noopNode : noopNode),
  },
);

const DOM = new Proxy(
  {
    entropy: { val: () => rolls },
    entropyMnemonicLength: { val: () => lengthSetting },
    phrase: { val: (value) => { phrase = value; } },
    // The tool reads the selected entropy base from a radio group.
    entropyTypeInputs: { filter: () => ({ val: () => "dice" }) },
  },
  {
    get: (target, key) => (key in target ? target[key] : noopNode),
  },
);

global.DOM = DOM;
global.entropyTypeAutoDetect = false;
global.mnemonic = mnemonic;
global.clearEntropyFeedback = () => {};
global.showEntropyFeedback = () => {};
global.writeSplitPhrase = () => {};
global.showWordIndexes = () => {};
global.showChecksum = () => {};

const filename = path.resolve(adapter);
vm.runInThisContext(
  `${fs.readFileSync(filename, "utf8")}\nsetMnemonicFromEntropy();`,
  { filename },
);

process.stdout.write(phrase === null ? "none\n" : `${phrase}\n`);
