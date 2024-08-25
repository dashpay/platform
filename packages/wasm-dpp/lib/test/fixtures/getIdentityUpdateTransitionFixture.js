const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');

const { IdentityUpdateTransition, IdentityPublicKeyWithWitness, default: loadWasmDpp } = require('../../..');

module.exports = async function getIdentityUpdateTransitionFixture() {
  await loadWasmDpp();

  const stateTransition = new IdentityUpdateTransition(1);
  stateTransition.setIdentityId(await generateRandomIdentifierAsync());

  const key = new IdentityPublicKeyWithWitness(1);
  key.setId(3);
  key.setData(Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'));

  stateTransition.setPublicKeysToAdd([key]);
  stateTransition.setPublicKeyIdsToDisable([0]);

  return stateTransition;
};
