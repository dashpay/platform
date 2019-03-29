const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');

const SVDocument = require('../stateView/document/SVDocument');

const WrongSequenceError = require('../blockchain/reader/eventHandlers/errors/WrongSequenceError');
const NotAbleToValidateSequenceError = require('../blockchain/reader/eventHandlers/errors/NotAbleToValidateSequenceError');

/**
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {Logger} logger
 */
module.exports = function attachSyncLogger(readerMediator, logger) {
  let isInitialSyncFinished = false;

  readerMediator.on(ReaderMediator.EVENTS.OUT_OF_BOUNDS, (params) => {
    const { initialBlockHeight, currentBlockCount } = params;

    logger.info(`Sync is not started due to initial block height ${initialBlockHeight} `
     + `is higher than the best block chain height ${currentBlockCount}`, {
      initialBlockHeight,
      currentBlockCount,
      event: ReaderMediator.EVENTS.OUT_OF_BOUNDS,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.FULLY_SYNCED, (currentBlockCount) => {
    isInitialSyncFinished = true;

    logger.info('Drive is fully synced', {
      currentBlockCount,
      event: ReaderMediator.EVENTS.FULLY_SYNCED,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BEGIN, (height) => {
    let message;

    if (!isInitialSyncFinished) {
      message = 'Start initial sync process';

      if (height === readerMediator.getInitialBlockHeight()) {
        message += ' from the initial block height';
      } else {
        message += ` from block ${height}`;
      }

      isInitialSyncFinished = true;
    } else {
      message = `Start sync process from ${height} block`;
    }

    logger.info(message, {
      height,
      initialBlockHeight: readerMediator.getInitialBlockHeight(),
      event: ReaderMediator.EVENTS.BEGIN,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_BEGIN, (block) => {
    logger.info(`Begin processing block ${block.height}`, {
      hash: block.hash,
      height: block.height,
      event: ReaderMediator.EVENTS.BLOCK_BEGIN,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_END, (block) => {
    logger.info(`End processing block ${block.height}`, {
      hash: block.hash,
      height: block.height,
      event: ReaderMediator.EVENTS.BLOCK_END,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE, (params) => {
    const { height, firstSyncedBlockHeight } = params;
    logger.info(`Block ${height} sequence can't be validated`
      + ` as its height less or equal first synced block ${firstSyncedBlockHeight}`, {
      height,
      firstSyncedBlockHeight,
      event: ReaderMediator.EVENTS.BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_ORPHANED, (block) => {
    logger.info(`Reverting orphaned block ${block.height}`, {
      hash: block.hash,
      height: block.height,
      previousBlockHash: block.previousblockhash,
      event: ReaderMediator.EVENTS.BLOCK_ORPHANED,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_ERROR, (params) => {
    const { error, block, stateTransition } = params;

    let loggingMethod = 'error';
    let loggingMessage = 'Error occurred during processing of a block '
      + `${block.height}: ${error.name} ${error.message}`;

    if (error instanceof WrongSequenceError || error instanceof NotAbleToValidateSequenceError) {
      loggingMethod = 'info';
      loggingMessage = `Incorrect sequence detected on a block: ${block.height}`;
    }

    logger[loggingMethod](
      loggingMessage,
      {
        block: {
          hash: block.hash,
          height: block.height,
        },
        error,
        stateTransition: (stateTransition ? { hash: stateTransition.hash } : null),
        event: ReaderMediator.EVENTS.BLOCK_ERROR,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION, (params) => {
    const { block, stateTransition } = params;
    logger.info(
      `Processing State Transition ${stateTransition.hash} for block ${block.height}`,
      {
        block: {
          hash: block.hash,
          height: block.height,
        },
        stateTransition: {
          hash: stateTransition.hash,
        },
        event: ReaderMediator.EVENTS.STATE_TRANSITION,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, (params) => {
    const { block, stateTransition } = params;
    logger.info(
      `Reverting orphaned State Transition ${stateTransition.hash} for block ${block.height}`,
      {
        block: {
          hash: block.hash,
          height: block.height,
        },
        stateTransition: {
          hash: stateTransition.hash,
        },
        event: ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_ERROR, (params) => {
    const { error, block, stateTransition } = params;
    logger.info(
      `Error processing State Transition ${stateTransition.hash} for block ${block.height}:`
        + ` ${error.name} ${error.message}`,
      {
        block: {
          hash: block.hash,
          height: block.height,
        },
        stateTransition: {
          hash: stateTransition.hash,
        },
        error,
        event: ReaderMediator.EVENTS.STATE_TRANSITION_ERROR,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_SKIP, (params) => {
    const { block, stateTransition } = params;
    logger.info(
      `Skipping processing of State Transition ${stateTransition.hash} for block ${block.height}`,
      {
        block: {
          hash: block.hash,
          height: block.height,
        },
        stateTransition: {
          hash: stateTransition.hash,
        },
        event: ReaderMediator.EVENTS.STATE_TRANSITION_SKIP,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.CONTRACT_APPLIED, (params) => {
    const { contractId, contract } = params;

    const contractAction = contract.version === 1 ? 'Created' : 'Updated';

    logger.info(
      `${contractAction} Contract ${contractId}`,
      {
        ...params,
        event: ReaderMediator.EVENTS.CONTRACT_APPLIED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.CONTRACT_REVERTED, (params) => {
    const {
      contractId,
      contract,
      previousRevision,
    } = params;

    logger.info(
      `Reverted Contract ${contractId} from ${contract.version} to ${previousRevision.getRevision()}`,
      {
        ...params,
        previousRevision: previousRevision.toJSON(),
        event: ReaderMediator.EVENTS.CONTRACT_REVERTED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.CONTRACT_MARKED_DELETED, (params) => {
    const { contractId } = params;

    logger.info(
      `Marked Contract for ${contractId} as deleted`,
      {
        ...params,
        event: ReaderMediator.EVENTS.CONTRACT_MARKED_DELETED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DOCUMENT_APPLIED, (params) => {
    const {
      contractId,
      documentId,
      document,
    } = params;

    const messages = {
      [SVDocument.ACTION_CREATE]: `Created Document ${documentId} for ${contractId}`,
      [SVDocument.ACTION_UPDATE]: `Updated Document ${documentId} for ${contractId}`,
      [SVDocument.ACTION_DELETE]: `Deleted Document ${documentId} for ${contractId}`,
    };

    const message = messages[document.act];

    logger.info(
      message,
      {
        ...params,
        event: ReaderMediator.EVENTS.DOCUMENT_APPLIED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DOCUMENT_REVERTED, (params) => {
    const {
      documentId,
      document,
      previousRevision,
    } = params;

    logger.info(
      `Reverted Document ${documentId} from ${document.rev} to ${previousRevision.getRevision()}`,
      {
        ...params,
        previousRevision: previousRevision.toJSON(),
        event: ReaderMediator.EVENTS.DOCUMENT_REVERTED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DOCUMENT_MARKED_DELETED, (params) => {
    const { documentId } = params;

    logger.info(
      `Document ${documentId} marked as deleted`,
      {
        ...params,
        event: ReaderMediator.EVENTS.DOCUMENT_MARKED_DELETED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.END, (readHeight) => {
    logger.info(`Sync process is finished on block ${readHeight}`, {
      readHeight,
      event: ReaderMediator.EVENTS.END,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.RESET, () => {
    const initialBlockHeight = readerMediator.getInitialBlockHeight();
    logger.info(
      `Cleanup Drive and restart sync process from initial block ${initialBlockHeight}`,
      {
        initialBlockHeight,
        event: ReaderMediator.EVENTS.RESET,
      },
    );
  });
};
