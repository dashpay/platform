const { default: loadWasmDpp } = require('../../..');
let { IdentityCreditTransferTransition } = require('../../..');
const generateRandomIdentifier = require('../utils/generateRandomIdentifierAsync');

module.exports = async function getIdentityUpdateTransitionFixture() {
  ({ IdentityCreditTransferTransition } = await loadWasmDpp());

  const rawStateTransition = {
    signature: Buffer.alloc(32).fill(0),
    signaturePublicKeyId: 0,
    protocolVersion: 1,
    type: 7,
    amount: 1000,
    identityId: (await generateRandomIdentifier()).toBuffer(),
    recipientId: (await generateRandomIdentifier()).toBuffer(),
  };

  return new IdentityCreditTransferTransition(rawStateTransition);
};
