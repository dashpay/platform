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
   * @param {ChainLockUpdate} coreChainLock
   * @return {Promise<boolean>}
   */
  async function verifyChainLock(coreChainLock) {
    const serializedCoreChainLock = {
      height: coreChainLock.coreBlockHeight,
      signature: Buffer.from(coreChainLock.signature).toString('hex'),
      blockHash: Buffer.from(coreChainLock.coreBlockHash).toString('hex'),
    };

    const lastCoreChainLockedHeight = latestBlockExecutionContext.getCoreChainLockedHeight();
    if (coreChainLock.coreBlockHeight <= lastCoreChainLockedHeight) {
      logger.debug(
        {
          chainLock: serializedCoreChainLock,
          lastCoreChainLockedHeight,
        },
        'Chainlock verification failed: coreBlockHeight must be bigger than the latest core chain locked height',
      );

      return false;
    }

    let isVerified;
    try {
      ({ result: isVerified } = await coreRpcClient.verifyChainLock(
        serializedCoreChainLock.blockHash,
        serializedCoreChainLock.signature,
        serializedCoreChainLock.height,
      ));
    } catch (e) {
      // Invalid signature format
      // Parse error
      if ([-8, -32700].includes(e.code)) {
        logger.debug(
          {
            err: e,
            chainLock: serializedCoreChainLock,
          },
          `Chainlock verification failed using verifyChainLock method: ${e.message} ${e.code}`,
        );

        return false;
      }

      throw e;
    }

    return isVerified;
  }

  return verifyChainLock;
}

module.exports = verifyChainLockFactory;
