const identityCreateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityCreate.json');

const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const Identity = require('@dashevo/dpp/lib/identity/Identity');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

let identity;

/**
 * @return {Identity}
 */
function getBiggestPossibleIdentity() {
  if (identity) {
    return identity;
  }

  const publicKeys = [];

  for (let i = 0; i < identityCreateTransitionSchema.properties.publicKeys.maxItems; i++) {
    const securityLevel = i === 0
      ? IdentityPublicKey.SECURITY_LEVELS.MASTER
      : IdentityPublicKey.SECURITY_LEVELS.HIGH;

    publicKeys.push({
      id: i,
      type: IdentityPublicKey.TYPES.BLS12_381,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel,
      readOnly: false,
      data: Buffer.alloc(48).fill(255),
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
