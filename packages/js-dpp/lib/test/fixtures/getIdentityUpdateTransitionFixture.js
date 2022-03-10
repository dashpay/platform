const protocolVersion = require('../../version/protocolVersion');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');
const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const IdentityUpdateTransition = require('../../identity/stateTransition/IdentityUpdateTransition/IdentityUpdateTransition');

module.exports = function getIdentityUpdateTransitionFixture() {
  const rawStateTransition = {
    protocolVersion: protocolVersion.latestVersion,
    type: stateTransitionTypes.IDENTITY_UPDATE,
    assetLockProof: getInstantAssetLockProofFixture().toObject(),
    identityId: generateRandomIdentifier(),
    revision: 0,
  };

  return new IdentityUpdateTransition(rawStateTransition);
};
