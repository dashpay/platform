const { default: loadWasmDpp } = require('../../..');
const { IdentityCreditTransferTransition } = require('../../..');
const generateRandomIdentifier = require('../utils/generateRandomIdentifierAsync');

module.exports = async function getIdentityUpdateTransitionFixture() {
  await loadWasmDpp();

  const stateTransition = new IdentityCreditTransferTransition(1);
  stateTransition.setAmount(1000);
  stateTransition.setIdentityId(await generateRandomIdentifier());
  stateTransition.setRecipientId(await generateRandomIdentifier());

  return stateTransition;
};
