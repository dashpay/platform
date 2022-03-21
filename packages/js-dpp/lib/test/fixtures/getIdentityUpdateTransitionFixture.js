const protocolVersion = require('../../version/protocolVersion');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');
const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const IdentityUpdateTransition = require('../../identity/stateTransition/IdentityUpdateTransition/IdentityUpdateTransition');
const IdentityPublicKey = require('../../identity/IdentityPublicKey');

module.exports = function getIdentityUpdateTransitionFixture() {
  const rawStateTransition = {
    protocolVersion: protocolVersion.latestVersion,
    type: stateTransitionTypes.IDENTITY_UPDATE,
    assetLockProof: getInstantAssetLockProofFixture().toObject(),
    identityId: generateRandomIdentifier(),
    revision: 0,
    addPublicKeys: [
      {
        id: 3,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
      },
    ],
    disablePublicKeys: [0],
    publicKeysDisabledAt: 1234567,
  };

  return new IdentityUpdateTransition(rawStateTransition);
};
