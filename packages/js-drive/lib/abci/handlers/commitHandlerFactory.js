const {
  tendermint: {
    abci: {
      ResponseCommit,
    },
  },
} = require('@dashevo/abci/types');

const { asValue } = require('awilix');

const DataCorruptedError = require('./errors/DataCorruptedError');
const BlockExecutionContextRepository = require('../../blockExecution/BlockExecutionContextRepository');

/**
 * @param {CreditsDistributionPool} creditsDistributionPool
 * @param {CreditsDistributionPoolCommonStoreRepository} creditsDistributionPoolRepository
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {DocumentDatabaseManager} documentDatabaseManager
 * @param {DocumentDatabaseManager} previousDocumentDatabaseManager
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {RootTree} rootTree
 * @param {
 * PreviousBlockExecutionStoreTransactionsRepository
 * } previousBlockExecutionStoreTransactionsRepository
 * @param {populateMongoDbTransactionFromObject} populateMongoDbTransactionFromObject
 * @param {AwilixContainer} container
 * @param {BaseLogger} logger
 * @param {cloneToPreviousStoreTransactions} cloneToPreviousStoreTransactions
 * @param {getLatestFeatureFlag} getLatestFeatureFlag
 * @param {RootTree} previousRootTree
 * @param {LRUCache} dataContractCache
 *
 * @return {commitHandler}
 */
function commitHandlerFactory(
  creditsDistributionPool,
  creditsDistributionPoolRepository,
  blockExecutionStoreTransactions,
  blockExecutionContext,
  previousBlockExecutionContext,
  blockExecutionContextRepository,
  documentDatabaseManager,
  previousDocumentDatabaseManager,
  transactionalDpp,
  rootTree,
  previousBlockExecutionStoreTransactionsRepository,
  populateMongoDbTransactionFromObject,
  container,
  logger,
  cloneToPreviousStoreTransactions,
  getLatestFeatureFlag,
  previousRootTree,
  dataContractCache,
) {
  /**
   * Commit ABCI Handler
   *
   * @typedef commitHandler
   *
   * @return {Promise<abci.ResponseCommit>}
   */
  async function commitHandler() {
    const { height: blockHeight } = blockExecutionContext.getHeader();

    const consensusLogger = logger.child({
      height: blockHeight.toString(),
      abciMethod: 'commit',
    });

    blockExecutionContext.setConsensusLogger(consensusLogger);

    consensusLogger.debug('Commit ABCI method requested');

    let nextPreviousBlockExecutionStoreTransactions;
    try {
      for (const dataContract of blockExecutionContext.getDataContracts()) {
        // Create document databases for dataContracts created in the current block
        await documentDatabaseManager.create(dataContract);

        // Update data contract cache with new version of
        // commited data contract
        const idString = dataContract.getId().toString();

        if (dataContractCache.has(idString)) {
          dataContractCache.set(idString, dataContract);
        }
      }

      // Store ST fees from the block to distribution pool
      creditsDistributionPool.incrementAmount(
        blockExecutionContext.getCumulativeFees(),
      );

      const commonStoreTransaction = blockExecutionStoreTransactions.getTransaction('common');
      await creditsDistributionPoolRepository.store(
        creditsDistributionPool,
        commonStoreTransaction,
      );

      // Clone changes from the current block to previous transactions
      nextPreviousBlockExecutionStoreTransactions = await cloneToPreviousStoreTransactions(
        blockExecutionStoreTransactions,
      );

      // Commit the current block db transactions
      await blockExecutionStoreTransactions.commit();

      // Store block execution contexts to external storage (outside of state trees, otherwise it
      // will change appHash even if we nave no transactions inside of the block)
      await blockExecutionContextRepository.store(
        BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
        blockExecutionContext,
      );

      if (!previousBlockExecutionContext.isEmpty()) {
        await blockExecutionContextRepository.store(
          BlockExecutionContextRepository.KEY_PREFIX_PREVIOUS,
          previousBlockExecutionContext,
        );
      }
    } catch (e) {
      // Abort DB transactions. It doesn't work since some of transaction may already be committed
      // and will produce "transaction in not started" error.
      if (blockExecutionStoreTransactions.isStarted()) {
        await blockExecutionStoreTransactions.abort();
      }

      // NOTE: we're calling drop only on the newly created data contracts
      // in case of contract update we keep any created data for now
      const newlyCreatedDataContracts = blockExecutionContext.getDataContracts()
        .filter((dataContract) => dataContract.getVersion() === 1);

      for (const dataContract of newlyCreatedDataContracts) {
        await documentDatabaseManager.drop(dataContract);
      }

      throw e;
    }

    // rebuild root tree with committed data from the current block
    rootTree.rebuild();

    // Commit previous block data to previous stores if available
    if (container.hasRegistration('previousBlockExecutionStoreTransactions')) {
      const previousBlockExecutionStoreTransactions = container.resolve(
        'previousBlockExecutionStoreTransactions',
      );

      // Create document databases in previous dbs
      const previousDataContractTransaction = previousBlockExecutionStoreTransactions.getTransaction('dataContracts');
      const { updates: previousCreatedDataContracts } = previousDataContractTransaction.toObject();

      const createDatabasePromises = Object.values(previousCreatedDataContracts)
        .map(async (serializedDataContract) => {
          const dataContract = await transactionalDpp.dataContract.createFromBuffer(
            serializedDataContract,
            {
              skipValidation: true,
            },
          );

          await previousDocumentDatabaseManager.create(dataContract);
        });

      await Promise.all(createDatabasePromises);

      // Create databases for documents
      const previousDocumentsTransaction = previousBlockExecutionStoreTransactions.getTransaction('documents');

      await populateMongoDbTransactionFromObject(
        previousDocumentsTransaction.getMongoDbTransaction(),
        previousDocumentsTransaction.toObject(),
      );

      // Commit previous block changes from the previous transactions to the previous stores
      await previousBlockExecutionStoreTransactions.commit();

      previousRootTree.rebuild();
    }

    // Update previous transactions with changes from the current block
    container.register({
      previousBlockExecutionStoreTransactions: asValue(nextPreviousBlockExecutionStoreTransactions),
    });

    // Persist previous transactions with changes from the previous block.
    // In case of failure the block won't be committed but the current state will be updated
    // since previous state won't have changes from H-1. info handler will provide
    // height of the current block so data from H-1 will be just lost in previous databases.
    try {
      await previousBlockExecutionStoreTransactionsRepository.store(
        nextPreviousBlockExecutionStoreTransactions,
      );
    } catch (e) {
      throw new DataCorruptedError(e);
    }

    const appHash = rootTree.getRootHash();

    consensusLogger.info(
      {
        appHash: appHash.toString('hex').toUpperCase(),
      },
      `Block commit #${blockHeight} with appHash ${appHash.toString('hex').toUpperCase()}`,
    );

    return new ResponseCommit({
      data: appHash,
    });
  }

  return commitHandler;
}

module.exports = commitHandlerFactory;
