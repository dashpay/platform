const { ChainLock } = require('@dashevo/dashcore-lib');

/**
 * @param {buffer} buffer - serialized chainlock as buffer
 * @return {ChainLock}
 */
function decodeChainLock(buffer) {
  return new ChainLock(buffer);
}

module.exports = decodeChainLock;
