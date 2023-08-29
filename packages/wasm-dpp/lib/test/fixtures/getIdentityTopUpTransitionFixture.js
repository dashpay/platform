const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');
const generateRandomIdentifier = require('../utils/generateRandomIdentifierAsync');

const { IdentityTopUpTransition, default: loadWasmDpp } = require('../../..');

/**
 * @param {PrivateKey} oneTimePrivateKey
 *
 * @return {IdentityCreateTransition}
 */
module.exports = async function getIdentityCreateTransitionFixture(
  oneTimePrivateKey = new PrivateKey(),
) {
  await loadWasmDpp();

  const stateTransition = new IdentityTopUpTransition(1);
  stateTransition.setIdentityId(await generateRandomIdentifier());
  stateTransition.setAssetLockProof(await getInstantAssetLockProofFixture(oneTimePrivateKey));

  return stateTransition;
};
