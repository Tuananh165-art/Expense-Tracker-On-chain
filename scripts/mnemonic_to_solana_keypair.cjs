#!/usr/bin/env node
const bip39 = require("bip39");
const { derivePath } = require("ed25519-hd-key");
const nacl = require("tweetnacl");
const bs58Module = require("bs58");
const bs58 = bs58Module.default ?? bs58Module;
const fs = require("fs");

function usage() {
  console.error(
    "Usage: node scripts/mnemonic_to_solana_keypair.cjs \"<12 words>\" [accountIndex]\n" +
      "Default path: m/44'/501'/0'/0'"
  );
  process.exit(1);
}

const mnemonic = process.argv[2];
const accountIndex = Number(process.argv[3] ?? 0);

if (!mnemonic || !bip39.validateMnemonic(mnemonic)) usage();
if (!Number.isInteger(accountIndex) || accountIndex < 0) usage();

const seed = bip39.mnemonicToSeedSync(mnemonic); // 64-byte seed
const path = `m/44'/501'/${accountIndex}'/0'`;   // Phantom/Solana phổ biến
const { key } = derivePath(path, seed.toString("hex")); // 32-byte private seed

const kp = nacl.sign.keyPair.fromSeed(key); // secretKey 64 bytes, publicKey 32 bytes
const secretKey = Buffer.from(kp.secretKey);
const publicKey = Buffer.from(kp.publicKey);

// in ra đủ format
console.log("derivation_path:", path);
console.log("public_key_base58:", bs58.encode(publicKey));
console.log("private_key_base58:", bs58.encode(secretKey)); // dùng cho nhiều lib/web3
console.log("private_key_json_array:", JSON.stringify(Array.from(secretKey)));

// optional: ghi file để dùng solana-keygen / script local
const out = `/tmp/wallet-derived-${accountIndex}.json`;
fs.writeFileSync(out, JSON.stringify(Array.from(secretKey)));
console.log("written_json:", out);
