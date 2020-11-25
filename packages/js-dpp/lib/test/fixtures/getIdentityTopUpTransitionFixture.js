const IdentityTopUpTransition = require('../../identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const getAssetLockFixture = require('./getAssetLockFixture');

/**
 *
 * @return {IdentityTopUpTransition}
 */
module.exports = function getIdentityTopUpTransitionFixture() {
  const rawStateTransition = {
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    assetLock: getAssetLockFixture().toObject(),
    identityId: generateRandomIdentifier(),
  };

  return new IdentityTopUpTransition(rawStateTransition);
};
