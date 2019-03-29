const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');

/**
 * Attach StateView handlers
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {applyStateTransition} applyStateTransition
 * @param {revertSVDocumentsForStateTransition} revertSVDocumentsForStateTransition
 * @param {revertSVContractsForStateTransition} revertSVContractsForStateTransition
 * @param {dropMongoDatabasesWithPrefix} dropMongoDatabasesWithPrefix
 * @param {string} mongoDbPrefix
 */
function attachStateViewHandlers(
  readerMediator,
  applyStateTransition,
  revertSVDocumentsForStateTransition,
  revertSVContractsForStateTransition,
  dropMongoDatabasesWithPrefix,
  mongoDbPrefix,
) {
  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION, async ({ stateTransition, block }) => {
    await applyStateTransition(stateTransition, block);
  });

  readerMediator.on(
    ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED,
    revertSVDocumentsForStateTransition,
  );

  readerMediator.on(
    ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED,
    revertSVContractsForStateTransition,
  );

  readerMediator.on(ReaderMediator.EVENTS.RESET, async () => {
    await dropMongoDatabasesWithPrefix(mongoDbPrefix);
  });
}

module.exports = attachStateViewHandlers;
