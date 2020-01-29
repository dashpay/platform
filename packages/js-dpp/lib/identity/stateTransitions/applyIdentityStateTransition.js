const Identity = require('../Identity');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

const WrongStateTransitionTypeError = require('../errors/WrongStateTransitionTypeError');
const IdentityAlreadyExistsError = require('../../errors/IdentityAlreadyExistsError');

/**
 * Applies a state transition to the identity model.
 * Only identity state transitions are allowed
 *
 * @param {IdentityCreateTransition} stateTransition
 * @param {Identity|null} identity
 * @return {Identity|null}
 */
function applyIdentityStateTransition(stateTransition, identity) {
  // noinspection JSRedundantSwitchStatement
  switch (stateTransition.getType()) {
    case stateTransitionTypes.IDENTITY_CREATE: {
      if (identity) {
        throw new IdentityAlreadyExistsError(stateTransition);
      }

      const newIdentity = new Identity({
        id: stateTransition.getIdentityId(),
        type: stateTransition.getIdentityType(),
      });

      newIdentity.setPublicKeys(stateTransition.getPublicKeys());

      return newIdentity;
    }
    default:
      throw new WrongStateTransitionTypeError(stateTransition);
  }
}

module.exports = applyIdentityStateTransition;
