const identityCreateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityCreate.json');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const {
  KeySecurityLevel, KeyType, KeyPurpose, Identity,
} = require('@dashevo/wasm-dpp');

let identity;

function getBiggestPossibleIdentity() {
  if (identity) {
    return identity;
  }

  const publicKeys = [];

  for (let i = 0; i < identityCreateTransitionSchema.properties.publicKeys.maxItems; i++) {
    const securityLevel = i === 0
      ? KeySecurityLevel.MASTER
      : KeySecurityLevel.HIGH;

    publicKeys.push({
      id: i,
      type: KeyType.BLS12_381,
      purpose: KeyPurpose.AUTHENTICATION,
      securityLevel,
      readOnly: false,
      data: Buffer.alloc(48).fill(i),
    });
  }

  identity = new Identity({
    protocolVersion: 1,
    id: generateRandomIdentifier().toBuffer(),
    publicKeys,
    balance: Math.floor(9223372036854775807 / 10000), // credits (i64) max + room for tests
    revision: Math.floor(18446744073709551615 / 10000), // u64 max + room for tests
  });

  return identity;
}

module.exports = getBiggestPossibleIdentity;
