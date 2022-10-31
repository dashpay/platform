const ChainlockVerificationFailedError = require('../../errors/ChainlockVerificationFailedError');

/**
 *
 * @param {RpcClient} coreRpcClient
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {BaseLogger} logger
 * @return {verifyChainLock}
 */
function verifyChainLockFactory(
  coreRpcClient,
  latestBlockExecutionContext,
  logger,
) {
  /**
   * @typedef verifyChainLock
   * @param {ChainLock} coreChainLock
   * @return {Promise<void>}
   */
  async function verifyChainLock(coreChainLock) {
    const lastCoreChainLockedHeight = latestBlockExecutionContext.getCoreChainLockedHeight();
    if (coreChainLock.coreBlockHeight <= lastCoreChainLockedHeight) {
      throw new ChainlockVerificationFailedError('coreBlockHeight is bigger than lastCoreChainLockedHeight', { chainlock: coreChainLock.toJSON(), lastCoreChainLockedHeight });
    }

    let isVerified;
    try {
      ({ result: isVerified } = await coreRpcClient.verifyChainLock(
        coreChainLock.coreBlockHash.toString('hex'),
        coreChainLock.signature.toString('hex'),
        coreChainLock.coreBlockHeight,
      ));
    } catch (e) {
      // Invalid signature format
      // Parse error
      if ([-8, -32700].includes(e.code)) {
        logger.debug(
          {
            err: e,
            chainLock: coreChainLock.toJSON(),
          },
          `Chainlock verification failed using verifyChainLock method: ${e.message} ${e.code}`,
        );

        throw new ChainlockVerificationFailedError(e.message, {
          chainlock: coreChainLock.toJSON(),
        });
      }

      throw e;
    }

    if (!isVerified) {
      logger.debug(`Invalid chainLock for height ${coreChainLock.coreBlockHeight}`);

      throw new ChainlockVerificationFailedError(
        'ChainLock is not valid', { chainlock: coreChainLock.toJSON() },
      );
    }

    logger.debug(`ChainLock is valid for height ${coreChainLock.coreBlockHeight}`);
  }

  return verifyChainLock;
}

module.exports = verifyChainLockFactory;
