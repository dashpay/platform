const identityCreateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityCreate.json');

const IdentityPublicKey = require('@dashevo/dpp/lib/Identity/IdentityPublicKey');

const Identity = require('@dashevo/dpp/lib/Identity/Identity');
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
    balance: Number.MAX_VALUE,
    revision: Number.MAX_VALUE,
  });

  return identity;
}

module.exports = getBiggestPossibleIdentity;
