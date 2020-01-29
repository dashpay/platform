const IdentityCreateTransition = require('../../identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');

const Identity = require('../../identity/Identity');
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
    identityType: Identity.TYPES.USER,
    publicKeys: [
      {
        id: 1,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.alloc(240).toString('base64'),
        isEnabled: true,
      },
    ],
  };

  return new IdentityCreateTransition(rawStateTransition);
};
