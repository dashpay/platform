/**
 * Get user object by revision.
 *
 * If it is current version then we obtain data just from state view.
 * Overwise we have to get data from storage (IPFS) also.
 *
 * @param {string} id Object ID
 * @param {string} revision Revision #
 * @return {UserObject}
 */
// eslint-disable-next-line no-unused-vars
module.exports = function getUserObjectByRevision(id, revision) {
  throw new Error('Not implemented yet');
};
