const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../..');
const { IdentityCreateTransition, IdentityPublicKey } = require('../../..');

/**
 * @param {PrivateKey} oneTimePrivateKey
 *
 * @return {IdentityCreateTransition}
 */
module.exports = async function getIdentityCreateTransitionFixture(
  oneTimePrivateKey = new PrivateKey(),
) {
  await loadWasmDpp();

  const assetLockProof = (await getInstantAssetLockProofFixture(oneTimePrivateKey)).toObject();
  assetLockProof.transaction = assetLockProof.transaction.toString('base64');
  assetLockProof.instantLock = assetLockProof.instantLock.toString('base64');
  // console.log(assetLockProof);
  const rawStateTransition = {
    signature: Buffer.alloc(0).toString('base64'),
    $version: '0',
    type: 2,
    assetLockProof,
    publicKeys: [
      // {
      //   id: 0,
      //   type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      //   data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
      //   purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      //   securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      //   readOnly: false,
      //   signature: Buffer.alloc(0),
      // },
    ],
  };
  console.log(rawStateTransition);

  return new IdentityCreateTransition(rawStateTransition);
};
