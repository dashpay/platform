const {
  tendermint: {
    abci: {
      ResponseEndBlock,
    },
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');

const NoDPNSContractFoundError = require('./errors/NoDPNSContractFoundError');
const NoDashpayContractFoundError = require('./errors/NoDashpayContractFoundError');

/**
 * Begin block ABCI handler
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {number|undefined} dpnsContractBlockHeight
 * @param {Identifier|undefined} dpnsContractId
 * @param {number|undefined} dashpayContractBlockHeight
 * @param {Identifier|undefined} dashpayContractId
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {BaseLogger} logger
 *
 * @return {endBlockHandler}
 */
function endBlockHandlerFactory(
  blockExecutionContext,
  dpnsContractBlockHeight,
  dpnsContractId,
  dashpayContractBlockHeight,
  dashpayContractId,
  latestCoreChainLock,
  validatorSet,
  createValidatorSetUpdate,
  logger,
) {
  /**
   * @typedef endBlockHandler
   *
   * @param {abci.RequestEndBlock} request
   * @return {Promise<abci.ResponseEndBlock>}
   */
  async function endBlockHandler(request) {
    const { height } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'endBlock',
    });

    consensusLogger.debug('EndBlock ABCI method requested');

    blockExecutionContext.setConsensusLogger(consensusLogger);

    if (dpnsContractId && height === dpnsContractBlockHeight) {
      if (!blockExecutionContext.hasDataContract(dpnsContractId)) {
        throw new NoDPNSContractFoundError(dpnsContractId, dpnsContractBlockHeight);
      }
    }

    if (dashpayContractId && height === dashpayContractBlockHeight) {
      if (!blockExecutionContext.hasDataContract(dashpayContractId)) {
        throw new NoDashpayContractFoundError(dashpayContractId, dashpayContractBlockHeight);
      }
    }

    const header = blockExecutionContext.getHeader();
    const lastCommitInfo = blockExecutionContext.getLastCommitInfo();
    const coreChainLock = latestCoreChainLock.getChainLock();

    // Rotate validators

    let validatorSetUpdate;
    const rotationEntropy = Buffer.from(lastCommitInfo.stateSignature);
    if (await validatorSet.rotate(height, coreChainLock.height, rotationEntropy)) {
      validatorSetUpdate = createValidatorSetUpdate(validatorSet);

      const { quorumHash } = validatorSet.getQuorum();

      consensusLogger.debug(
        {
          quorumHash,
        },
        `Validator Set switched to ${quorumHash} quorum`,
      );
    }

    // Update Core Chain Locks

    let nextCoreChainLockUpdate;
    if (coreChainLock && coreChainLock.height > header.coreChainLockedHeight) {
      nextCoreChainLockUpdate = new CoreChainLock({
        coreBlockHeight: coreChainLock.height,
        coreBlockHash: coreChainLock.blockHash,
        signature: coreChainLock.signature,
      });

      consensusLogger.trace(
        {
          nextCoreChainLockHeight: coreChainLock.height,
        },
        `Provide next chain lock for Core height ${coreChainLock.height}`,
      );
    }

    const validTxCount = blockExecutionContext.getValidTxCount();
    const invalidTxCount = blockExecutionContext.getInvalidTxCount();

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Block end #${height} (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount})`,
    );

    return new ResponseEndBlock({
      validatorSetUpdate,
      nextCoreChainLockUpdate,
    });
  }

  return endBlockHandler;
}

module.exports = endBlockHandlerFactory;
