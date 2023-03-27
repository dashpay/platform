const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../..');
let { IdentityCreateTransition, IdentityPublicKey } = require('../../..');

/**
 * @param {PrivateKey} oneTimePrivateKey
 *
 * @return {IdentityCreateTransition}
 */
module.exports = async function getIdentityCreateTransitionFixture(
  oneTimePrivateKey = new PrivateKey(),
) {
  ({ IdentityCreateTransition, IdentityPublicKey } = await loadWasmDpp());
  const rawStateTransition = {
    protocolVersion: 1,
    type: 2,
    assetLockProof: (await getInstantAssetLockProofFixture(oneTimePrivateKey)).toObject(),
    publicKeys: [
      {
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
        signature: Buffer.alloc(0),
      },
    ],
  };

  return new IdentityCreateTransition(rawStateTransition);
};
