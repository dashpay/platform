const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');
const generateRandomIdentifier = require('../utils/generateRandomIdentifierAsync');

const { default: loadWasmDpp } = require('../../..');
let { IdentityTopUpTransition } = require('../../..');

/**
 * @param {PrivateKey} oneTimePrivateKey
 *
 * @return {IdentityCreateTransition}
 */
module.exports = async function getIdentityCreateTransitionFixture(
  oneTimePrivateKey = new PrivateKey(),
) {
  ({ IdentityTopUpTransition } = await loadWasmDpp());
  const rawStateTransition = {
    $version: '0',
    signature: Buffer.alloc(32),
    type: 3,
    assetLockProof: (await getInstantAssetLockProofFixture(oneTimePrivateKey)).toObject(),
    identityId: await generateRandomIdentifier(),
  };

  return new IdentityTopUpTransition(rawStateTransition);
};
