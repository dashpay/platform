const IdentityTopUpTransition = require('../../identity/stateTransition/IdentityTopUpTransition/IdentityTopUpTransition');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const getInstantAssetLockProofFixture = require('./getInstantAssetLockProofFixture');

const protocolVersion = require('../../version/protocolVersion');

/**
 *
 * @return {IdentityTopUpTransition}
 */
module.exports = function getIdentityTopUpTransitionFixture() {
  const rawStateTransition = {
    protocolVersion: protocolVersion.latestVersion,
    type: stateTransitionTypes.IDENTITY_CREATE,
    assetLockProof: getInstantAssetLockProofFixture().toObject(),
    identityId: generateRandomIdentifier(),
  };

  return new IdentityTopUpTransition(rawStateTransition);
};
