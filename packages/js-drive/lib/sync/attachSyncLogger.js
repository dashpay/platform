const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');

const DapObject = require('../stateView/dapObject/DapObject');

const doubleSha256 = require('../util/doubleSha256');

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

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_STALE, (block) => {
    logger.info(`Reverting stale block ${block.height}`, {
      hash: block.hash,
      height: block.height,
      previousBlockHash: block.previousblockhash,
      event: ReaderMediator.EVENTS.BLOCK_STALE,
    });
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_ERROR, (params) => {
    const { error, block, stateTransition } = params;
    logger.error(
      `Error occurred during processing of a block ${block.height}: ${error.name} ${error.message}`,
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

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_STALE, (params) => {
    const { block, stateTransition } = params;
    logger.info(
      `Reverting stale State Transition ${stateTransition.hash} for block ${block.height}`,
      {
        block: {
          hash: block.hash,
          height: block.height,
        },
        stateTransition: {
          hash: stateTransition.hash,
        },
        event: ReaderMediator.EVENTS.STATE_TRANSITION_STALE,
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

  readerMediator.on(ReaderMediator.EVENTS.DAP_CONTRACT_APPLIED, (params) => {
    const { dapId, contract } = params;

    const contractAction = dapId === doubleSha256(contract) ? 'Created' : 'Updated';

    logger.info(
      `${contractAction} DAP Contract ${dapId}`,
      {
        ...params,
        event: ReaderMediator.EVENTS.DAP_CONTRACT_APPLIED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DAP_CONTRACT_REVERTED, (params) => {
    const {
      dapId,
      contract,
      previousVersion,
    } = params;

    logger.info(
      `Reverted DAP Contract ${dapId} from ${contract.dapver} to ${previousVersion.version}`,
      {
        ...params,
        event: ReaderMediator.EVENTS.DAP_CONTRACT_REVERTED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DAP_CONTRACT_MARKED_DELETED, (params) => {
    const { dapId } = params;

    logger.info(
      `Marked DAP Contract for ${dapId} as deleted`,
      {
        ...params,
        event: ReaderMediator.EVENTS.DAP_CONTRACT_MARKED_DELETED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DAP_OBJECT_APPLIED, (params) => {
    const {
      dapId,
      objectId,
      object,
    } = params;

    const messages = {
      [DapObject.ACTION_CREATE]: `Created DAP Object ${objectId} for ${dapId}`,
      [DapObject.ACTION_UPDATE]: `Updated DAP Object ${objectId} for ${dapId}`,
      [DapObject.ACTION_DELETE]: `Deleted DAP Object ${objectId} for ${dapId}`,
    };

    const message = messages[object.act];

    logger.info(
      message,
      {
        ...params,
        event: ReaderMediator.EVENTS.DAP_OBJECT_APPLIED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DAP_OBJECT_REVERTED, (params) => {
    const {
      objectId,
      object,
      previousRevision,
    } = params;

    logger.info(
      `Reverted DAP Object ${objectId} from ${object.rev} to ${previousRevision.revision}`,
      {
        ...params,
        event: ReaderMediator.EVENTS.DAP_OBJECT_REVERTED,
      },
    );
  });

  readerMediator.on(ReaderMediator.EVENTS.DAP_OBJECT_MARKED_DELETED, (params) => {
    const { objectId } = params;

    logger.info(
      `DAP Object ${objectId} marked as deleted`,
      {
        ...params,
        event: ReaderMediator.EVENTS.DAP_OBJECT_MARKED_DELETED,
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
