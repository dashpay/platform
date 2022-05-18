const IdentityPublicKey = require('./IdentityPublicKey');
const Identity = require('./Identity');

let identity;

function getBiggestPossibleIdentity() {
  if (identity) {
    return identity;
  }

  const publicKeys = [];

  for (let i = 0; i < 25; i++) {
    publicKeys.push({
      id: i,
      type: 1,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: true,
      // Copy data buffer
      data: Buffer.alloc(48).fill(255),
    });
  }

  identity = new Identity({
    protocolVersion: 1,
    id: Buffer.alloc(32).fill(255),
    publicKeys,
    balance: Number.MAX_VALUE,
    revision: Number.MAX_VALUE,
  });

  return identity;
}

module.exports = getBiggestPossibleIdentity;
