// Shared testnet fixtures used by functional tests.
// Values are sourced from the wasm-sdk docs generator and api-definitions
// to exercise read-only queries against publicly available testnet data.

export const TEST_IDS = {
  identityId: '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
  specializedBalanceIdentityId: 'AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT',
  dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
  tokenContractId: 'ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A',
  groupContractId: '49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N',
  tokenId: 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv',
  documentType: 'domain',
  documentId: '7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r',
  proTxHash: '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113',
  publicKeyHashUnique: 'b7e904ce25ed97594e72f7af0e66f298031c1754',
  publicKeyHashNonUnique: '518038dc858461bcee90478fd994bba8057b7531',
  username: 'alice',
  existingUsername: 'therealslimshaddy5',
  epoch: 8635,
  // Platform address fixtures (21 bytes: type byte + 20-byte hash)
  // Type 0x00 = P2PKH, Type 0x01 = P2SH
  // These are test addresses with zero hashes - queries will return undefined if not funded
  platformAddressBytes: new Uint8Array([
    0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  ]),
  platformAddressesBytesArray: [
    new Uint8Array([0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    new Uint8Array([0x00, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
  ],
};

// Optional environment-driven secrets for state-transition tests (skipped by default).
export const TEST_SECRETS = {
  identityId: process.env.EVO_IDENTITY_ID,
  privateKeyWif: process.env.EVO_PRIVATE_WIF,
  keyId: process.env.EVO_KEY_ID ? Number(process.env.EVO_KEY_ID) : undefined,
};
