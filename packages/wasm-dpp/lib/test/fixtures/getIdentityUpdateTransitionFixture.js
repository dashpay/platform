const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../..');
let { IdentityUpdateTransition, IdentityPublicKey } = require('../../..');

module.exports = async function getIdentityUpdateTransitionFixture() {
  ({ IdentityUpdateTransition, IdentityPublicKey } = await loadWasmDpp());

  const rawStateTransition = {
    $version: '0',
    signature: Buffer.alloc(32),
    signaturePublicKeyId: 0,
    type: 5,
    assetLockProof: (await getInstantAssetLockProofFixture()).toObject(),
    identityId: (await generateRandomIdentifierAsync()).toBuffer(),
    revision: 0,
    addPublicKeys: [
      {
        $version: '0',
        id: 3,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        signature: Buffer.alloc(32),
        readOnly: false,
      },
    ],
    disablePublicKeys: [0],
    publicKeysDisabledAt: 1234567,
  };

  return new IdentityUpdateTransition(rawStateTransition);
};
