/**
 * Get DAP Schema by version.
 *
 * If it is current version then we obtain data just from state view.
 * Overwise we have to get data from storage (IPFS) also.
 *
 * @param {string} dapId DAP ID
 * @param {string} version DAP version
 * @return {DAPSchema}
 */
// eslint-disable-next-line no-unused-vars
module.exports = function getDapSchemaByVersion(dapId, version) {
  throw new Error('Not implemented yet');
};
