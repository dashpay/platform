const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../..');
const { IdentityCreateTransition, IdentityPublicKeyWithWitness } = require('../../..');

/**
 * @param {PrivateKey} oneTimePrivateKey
 *
 * @return {IdentityCreateTransition}
 */
module.exports = async function getIdentityCreateTransitionFixture(
  oneTimePrivateKey = new PrivateKey(),
) {
  await loadWasmDpp();

  const assetLockProof = await getInstantAssetLockProofFixture(oneTimePrivateKey);

  const stateTransition = new IdentityCreateTransition(1);
  stateTransition.setAssetLockProof(assetLockProof);

  const publicKey = new IdentityPublicKeyWithWitness(1);
  publicKey.setData(Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'));

  stateTransition.setPublicKeys([publicKey]);

  return stateTransition;
};
