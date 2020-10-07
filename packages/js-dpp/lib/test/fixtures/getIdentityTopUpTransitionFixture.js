const bs58 = require('bs58');
const IdentityTopUpTransition = require('../../identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

/**
 *
 * @return {IdentityTopUpTransition}
 */
module.exports = function getIdentityTopUpTransitionFixture() {
  const rawStateTransition = {
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    lockedOutPoint: Buffer.alloc(36),
    identityId: bs58.decode('9egkkRs6ErFbLUh3yYn8mdgeKGpJQ41iayS1Z9bwsRM7'),
  };

  return new IdentityTopUpTransition(rawStateTransition);
};
