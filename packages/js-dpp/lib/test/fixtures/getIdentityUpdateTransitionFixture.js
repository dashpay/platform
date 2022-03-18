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
        id: 1,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
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
