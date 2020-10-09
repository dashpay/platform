const IdentityTopUpTransition = require('../../identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

/**
 *
 * @return {IdentityTopUpTransition}
 */
module.exports = function getIdentityTopUpTransitionFixture() {
  const rawStateTransition = {
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    lockedOutPoint: Buffer.alloc(36).fill('x'),
    identityId: generateRandomIdentifier(),
  };

  return new IdentityTopUpTransition(rawStateTransition);
};
