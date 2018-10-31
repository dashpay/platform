const ReaderMediator = require('./BlockchainReaderMediator');

/**
 * @param {BlockchainReader} reader
 * @param {BlockchainReaderMediator} readerMediator
 * @param {RpcClient} rpcClient
 * @return {readBlockchain}
 */
module.exports = function readBlockchainFactory(reader, readerMediator, rpcClient) {
  /**
   * @typedef readBlockchain
   * @return {Promise<void>}
   */
  async function readBlockchain() {
    const { result: currentBlockCount } = await rpcClient.getBlockCount();
    const lastSyncedBlock = readerMediator.getState().getLastBlock();

    const initialBlockHeight = readerMediator.getInitialBlockHeight();

    // Start sync with initial block height (first Evo block)
    let height = initialBlockHeight;

    // Skip syncing if current height less than initial block height
    if (height > currentBlockCount) {
      await readerMediator.emitSerial(ReaderMediator.EVENTS.OUT_OF_BOUNDS, {
        initialBlockHeight: height,
        currentBlockCount,
      });

      // Clean state if we have synced blocks
      if (lastSyncedBlock) {
        await readerMediator.reset();
      }

      return;
    }

    // Already synced something
    if (lastSyncedBlock) {
      // Drive is fully synced if last synced block height
      // and hash are the same with the last block from chain
      if (lastSyncedBlock.height === currentBlockCount) {
        const { result: blockHash } = await rpcClient.getBlockHash(currentBlockCount);
        if (lastSyncedBlock.hash === blockHash) {
          await readerMediator.emitSerial(
            ReaderMediator.EVENTS.FULLY_SYNCED,
            currentBlockCount,
          );

          return;
        }
      }

      if (lastSyncedBlock.height > currentBlockCount) {
        // Sync since current chain height if last synced block is higher (reorg)
        height = currentBlockCount;

        const firstSyncedBlockHeight = readerMediator.getState().getFirstBlockHeight();

        if (height <= firstSyncedBlockHeight) {
          await readerMediator.emitSerial(
            ReaderMediator.EVENTS.BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE,
            {
              height,
              firstSyncedBlockHeight,
            },
          );

          // The state does not have previous block to rely onto
          await readerMediator.reset();

          // Start reading from initial block height
          height = initialBlockHeight;
        }
      } else {
        // Sync the next block if last synced block height less then current height
        height = lastSyncedBlock.height + 1;
      }
    }

    await readerMediator.emitSerial(
      ReaderMediator.EVENTS.BEGIN,
      height,
    );

    const lastSyncedHeight = await reader.read(height);

    await readerMediator.emitSerial(
      ReaderMediator.EVENTS.END,
      lastSyncedHeight,
    );
  }

  return readBlockchain;
};
