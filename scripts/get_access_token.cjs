const fs = require("fs");
const nacl = require("tweetnacl");
const bs58Module = require("bs58");
const bs58 = bs58Module.default ?? bs58Module;

(async () => {
  const api = process.env.API_BASE_URL || "http://localhost:8080";
  const keyFile = process.env.SOLANA_KEYPAIR_PATH || "/tmp/wallet-2ksv.json";

  const secret = Uint8Array.from(JSON.parse(fs.readFileSync(keyFile, "utf8"))); // 64 bytes
  const pub = secret.slice(32, 64);
  const wallet = bs58.encode(Buffer.from(pub));

  const chRes = await fetch(`${api}/api/v1/auth/challenge`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ wallet_address: wallet }),
  });
  if (!chRes.ok) throw new Error(`challenge failed: ${chRes.status} ${await chRes.text()}`);
  const ch = await chRes.json();

  const sig = nacl.sign.detached(Buffer.from(ch.message, "utf8"), secret);
  const sig58 = bs58.encode(Buffer.from(sig));

  const vRes = await fetch(`${api}/api/v1/auth/verify`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      challenge_id: ch.challenge_id,
      wallet_address: wallet,
      signature: sig58,
    }),
  });
  if (!vRes.ok) throw new Error(`verify failed: ${vRes.status} ${await vRes.text()}`);
  const v = await vRes.json();

  process.stdout.write(v.access_token); // in raw token
})().catch((e) => {
  console.error(e.message || e);
  process.exit(1);
});
