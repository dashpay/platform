// Shared local fixtures for functional tests against dashmate local network seeded via SDK_TEST_DATA.

export const TEST_IDS = {
  // Seeded identity (32 bytes of 0x01)
  identityId: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi',
  specializedBalanceIdentityId: '8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR',
  dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', // DPNS
  tokenContractId: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8', // seeded token contract (0x03)
  groupContractId: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8', // same as seeded token contract
  tokenId: 'DdNpSsShdZJnKBb6njFm6eTLSaj4AcQdKZJPDEUK4w49', // tokenId for position 0 of tokenContractId
  documentType: 'domain',
  documentId: null,
  proTxHash: null,
  publicKeyHashUnique: null,
  publicKeyHashNonUnique: null,
  username: 'therealslimshaddy5',
  existingUsername: 'therealslimshaddy5',
  epoch: null,
};

export const TEST_SECRETS = {
  identityId: process.env.EVO_IDENTITY_ID,
  privateKeyWif: process.env.EVO_PRIVATE_WIF,
  keyId: process.env.EVO_KEY_ID ? Number(process.env.EVO_KEY_ID) : undefined,
};
