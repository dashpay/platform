const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {decodeChainLock} decodeChainLock
 * @return {verifyChainLockQueryHandler}
 */
function verifyChainLockQueryHandlerFactory(simplifiedMasternodeList, decodeChainLock) {
  /**
   * @typedef verifyChainLockQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer} data.chainLock
   * @return {Promise<ResponseQuery>}
   */
  async function verifyChainLockQueryHandler(params, data) {
    const chainlock = decodeChainLock(data.chainLock);

    if (!chainlock.verify(simplifiedMasternodeList.getStore())) {
      throw new InvalidArgumentAbciError(
        'Signature invalid for chainlock', chainlock.toJSON(),
      );
    }

    return new ResponseQuery();
  }

  return verifyChainLockQueryHandler;
}

module.exports = verifyChainLockQueryHandlerFactory;
