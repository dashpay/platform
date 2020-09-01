/**
 * Return best block height
 * @return {number}
 */
module.exports = async function getBestBlockHeightFromTransport() {
  return this.transport.getBestBlockHeight();
};
