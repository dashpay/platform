const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');

/**
 * Attach StateView handlers
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {applyStateTransition} applyStateTransition
 * @param {revertDapObjectsForStateTransition} revertDapObjectsForStateTransition
 * @param {revertDapContractsForStateTransition} revertDapContractsForStateTransition
 * @param {dropMongoDatabasesWithPrefix} dropMongoDatabasesWithPrefix
 * @param {string} mongoDbPrefix
 */
function attachStateViewHandlers(
  readerMediator,
  applyStateTransition,
  revertDapObjectsForStateTransition,
  revertDapContractsForStateTransition,
  dropMongoDatabasesWithPrefix,
  mongoDbPrefix,
) {
  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION, async ({ stateTransition, block }) => {
    await applyStateTransition(stateTransition, block);
  });

  readerMediator.on(
    ReaderMediator.EVENTS.STATE_TRANSITION_STALE,
    revertDapObjectsForStateTransition,
  );

  readerMediator.on(
    ReaderMediator.EVENTS.STATE_TRANSITION_STALE,
    revertDapContractsForStateTransition,
  );

  readerMediator.on(ReaderMediator.EVENTS.RESET, async () => {
    await dropMongoDatabasesWithPrefix(mongoDbPrefix);
  });
}

module.exports = attachStateViewHandlers;
