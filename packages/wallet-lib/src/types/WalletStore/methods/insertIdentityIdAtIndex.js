const IdentityReplaceError = require('../../../errors/IndentityIdReplaceError');

function insertIdentityIdAtIndex(identityId, identityIndex) {
  const existingId = this.getIdentityIdByIndex(identityIndex);

  if (Boolean(existingId) && existingId !== identityId) {
    throw new IdentityReplaceError(`Trying to replace identity at index ${identityIndex}`);
  }

  this.state.identities.set(identityIndex, identityId);
}
module.exports = insertIdentityIdAtIndex;
