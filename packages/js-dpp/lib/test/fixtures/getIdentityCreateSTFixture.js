const IdentityCreateTransition = require('../../identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');

const IdentityPublicKey = require('../../identity/IdentityPublicKey');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

/**
 *
 * @return {IdentityCreateTransition}
 */
module.exports = function getIdentityCreateSTFixture() {
  const rawStateTransition = {
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    lockedOutPoint: Buffer.alloc(36).toString('base64'),
    publicKeys: [
      {
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di',
        isEnabled: true,
      },
    ],
  };

  return new IdentityCreateTransition(rawStateTransition);
};
