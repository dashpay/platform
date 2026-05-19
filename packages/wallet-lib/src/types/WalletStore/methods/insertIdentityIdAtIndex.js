import IdentityReplaceError from '../../../errors/IndentityIdReplaceError.js';

function insertIdentityIdAtIndex(identityId, identityIndex) {
  const existingId = this.getIdentityIdByIndex(identityIndex);

  if (Boolean(existingId) && existingId !== identityId) {
    throw new IdentityReplaceError(`Trying to replace identity at index ${identityIndex}`);
  }

  this.state.identities.set(identityIndex, identityId);
}
export default insertIdentityIdAtIndex;