const { EventEmitter } = require('events');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const { createBloomFilter, parseRawTransactions, parseRawMerkleBlock } = require('./utils');

const logger = require('../../../logger');

const EVENTS = {
  HISTORICAL_TRANSACTIONS: 'HISTORICAL_TRANSACTIONS',
  NEW_TRANSACTIONS: 'NEW_TRANSACTIONS',
  MERKLE_BLOCK: 'MERKLE_BLOCK',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
  STOPPED: 'STOPPED',
};

/**
 * @typedef TransactionsReaderOptions
 * @property {Function} [createHistoricalSyncStream]
 * @property {Function} [createContinuousSyncStream]
 * @property {string} [network]
 * @property {number} maxRetries
 */

/**
 * @typedef TransactionsReader
 * @extends {EventEmitter}
 * @class
 */
class TransactionsReader extends EventEmitter {
  /**
   * @param {TransactionsReaderOptions} options
   */
  constructor(options = {}) {
    super();
    this.createHistoricalSyncStream = options.createHistoricalSyncStream;
    this.createContinuousSyncStream = options.createContinuousSyncStream;
    this.network = options.network;
    this.maxRetries = options.maxRetries;

    this.historicalSyncStream = null;
    this.continuousSyncStream = null;
  }

  /**
   * Reads historical transactions
   *
   * @param {number} fromBlockHeight
   * @param {number} toBlockHeight
   * @param {string[]} addresses
   * @returns {Promise<void>}
   */
  async startHistoricalSync(fromBlockHeight, toBlockHeight, addresses) {
    if (this.historicalSyncStream) {
      throw new Error('Historical sync is already in process');
    }

    if (!addresses || addresses.length === 0) {
      throw new Error('No addresses to sync');
    }

    if (fromBlockHeight < 1) {
      throw new Error(`Invalid fromBlockHeight: ${fromBlockHeight}`);
    }

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount <= 0) {
      throw new Error(`Invalid total amount of blocks to sync: ${totalAmount}`);
    }

    const subscribeWithRetries = this.subscribeToHistoricalBatch(this.maxRetries);
    const count = toBlockHeight - fromBlockHeight + 1;
    this.historicalSyncStream = await subscribeWithRetries(fromBlockHeight, count, addresses);
    logger.debug(`[TransactionsReader] Started syncing blocks from ${fromBlockHeight} to ${toBlockHeight}`);
  }

  /**
   * A HOF that returns a function to subscribe to historical block headers and chain locks
   * and handles retry logic
   *
   * @private
   * @param {number} [maxRetries=0] - maximum amount of retries
   * @returns {function(*, *, *): Promise<Stream>}
   */
  subscribeToHistoricalBatch(maxRetries = 0) {
    let currentRetries = 0;
    /**
     * Subscribes to the stream of historical data and handles retry logic
     *
     * @param {number} fromBlockHeight
     * @param {number} count
     * @param {string[]} addresses
     * @return Promise<!grpc.web.ClientReadableStream>
     */
    const subscribeWithRetries = async (fromBlockHeight, count, addresses) => {
      let lastSyncedBlockHeight = fromBlockHeight - 1;
      const bloomFilter = createBloomFilter(addresses);
      const stream = await this.createHistoricalSyncStream(bloomFilter, {
        fromBlockHeight,
        count,
      });
      this.historicalSyncStream = stream;

      // Arguments for the stream restart when it comes to a need to expand bloom filter
      let restartArgs = null;

      const dataHandler = (data) => {
        const rawTransactions = data.getRawTransactions();
        // TBD: const rawInstantLocks = data.getInstantSendLockMessages();
        const rawMerkleBlock = data.getRawMerkleBlock();

        if (rawTransactions) {
          const transactions = parseRawTransactions(rawTransactions, addresses, this.network);

          if (transactions.length) {
            this.emit(EVENTS.HISTORICAL_TRANSACTIONS, transactions);
          }
        } else if (rawMerkleBlock) {
          const merkleBlock = parseRawMerkleBlock(rawMerkleBlock);

          let rejected = false;
          let accepted = false;

          /**
           * Accepts merkle block
           * @param {number} height
           * @param {string[]} newAddresses
           */
          const acceptMerkleBlock = (height, newAddresses) => {
            if (rejected) {
              throw new Error('Unable to accept rejected merkle block');
            }

            lastSyncedBlockHeight = height;
            const blocksRead = lastSyncedBlockHeight - fromBlockHeight + 1;
            const remainingCount = count - blocksRead;

            if (remainingCount === 0) {
              return;
            }

            if (remainingCount < 0) {
              throw new Error(`Merkle block height is greater than expected range: ${height} > ${fromBlockHeight + count - 1}`);
            }

            if (newAddresses.length) {
              restartArgs = {
                fromBlockHeight: height + 1,
                count: remainingCount,
                addresses: [...addresses, ...newAddresses],
              };

              // Restart stream to expand bloom filter
              stream.cancel();
            }

            accepted = true;
          };

          const rejectMerkleBlock = (e) => {
            if (accepted) {
              throw new Error('Unable to reject accepted merkle block');
            }
            rejected = true;
            stream.destroy(e);
          };

          this.emit(EVENTS.MERKLE_BLOCK, { merkleBlock, acceptMerkleBlock, rejectMerkleBlock });
        }
      };

      const errorHandler = (streamError) => {
        if (streamError.code === GrpcErrorCodes.CANCELLED) {
          // TODO: consider reworking with COMMANDS instead
          // of producing a side effect that alters class state
          this.historicalSyncStream = null;
          if (restartArgs) {
            logger.debug('[TransactionsReader] Restarting stream with', {
              fromBlockHeight: restartArgs.fromBlockHeight,
              count: restartArgs.count,
              _addressesCount: addresses.length,
            });
            subscribeWithRetries(
              restartArgs.fromBlockHeight,
              restartArgs.count,
              restartArgs.addresses,
            ).then((newStream) => {
              this.historicalSyncStream = newStream;
            }).catch((e) => {
              this.emit(EVENTS.ERROR, e);
            }).finally(() => {
              restartArgs = null;
            });
          }

          logger.debug('[TransactionsReader] Stream canceled on client');

          return;
        }

        if (currentRetries < maxRetries) {
          logger.debug(`[TransactionsReader] Stream error, retry attempt ${currentRetries}/${maxRetries}`, streamError);
          if (!restartArgs) {
            const blocksRead = lastSyncedBlockHeight - fromBlockHeight + 1;
            const remainingCount = count - blocksRead;
            if (remainingCount <= 0) {
              return;
            }

            restartArgs = {
              fromBlockHeight: lastSyncedBlockHeight + 1,
              count: remainingCount,
              addresses,
            };
          }

          subscribeWithRetries(
            restartArgs.fromBlockHeight,
            restartArgs.count,
            restartArgs.addresses,
          ).then((newStream) => {
            this.historicalSyncStream = newStream;
            currentRetries += 1;
          }).catch((e) => {
            this.emit(EVENTS.ERROR, e);
          }).finally(() => {
            restartArgs = null;
          });
        } else {
          this.emit(EVENTS.ERROR, streamError);
        }
      };

      const endHandler = () => {
        if (!restartArgs) {
          logger.debug('[----------------->] Historical data updated');
          this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
        }
      };

      stream.on('data', dataHandler);
      stream.on('error', errorHandler);
      stream.on('end', endHandler);

      return stream;
    };

    return subscribeWithRetries;
  }

  /**
   * Creates continuous stream to read transactions
   * @param {number} fromBlockHeight
   * @param {string[]} addresses
   * @return {Promise<void>}
   */
  async startContinuousSync(fromBlockHeight, addresses) {
    if (this.continuousSyncStream) {
      throw new Error('Continuous sync has already been started');
    }

    if (!Array.isArray(addresses)) {
      throw new Error(`Invalid addresses: ${addresses}`);
    }

    if (addresses.length === 0) {
      throw new Error('Empty addresses list provided');
    }

    if (fromBlockHeight < 1) {
      throw new Error(`Invalid fromBlockHeight: ${fromBlockHeight}`);
    }

    const bloomFilter = createBloomFilter(addresses);
    const stream = await this.createContinuousSyncStream(bloomFilter, {
      fromBlockHeight,
      count: 0,
    });
    this.continuousSyncStream = stream;

    logger.debug('[TransactionsReader] Started continuous sync with', {
      fromBlockHeight,
      _addressesCount: addresses.length,
    });

    // Arguments for the stream restart when it comes to a need to expand bloom filter
    let restartArgs = null;
    let addressesGenerated = [];
    let lastSyncedBlockHeight = fromBlockHeight;

    const dataHandler = (data) => {
      const rawTransactions = data.getRawTransactions();
      // TBD: const rawInstantLocks = data.getInstantSendLockMessages();
      const rawMerkleBlock = data.getRawMerkleBlock();

      if (rawTransactions) {
        const transactions = parseRawTransactions(rawTransactions, addresses, this.network);

        /**
         * @param {string[]} newAddresses
         */
        const appendAddresses = (newAddresses) => {
          addressesGenerated = [...addressesGenerated, ...newAddresses];
        };

        if (transactions.length) {
          this.emit(EVENTS.NEW_TRANSACTIONS, { transactions, appendAddresses });
        }
      } else if (rawMerkleBlock) {
        const merkleBlock = parseRawMerkleBlock(rawMerkleBlock);

        let rejected = false;
        let accepted = false;

        /**
         * Accepts merkle block
         * @param {number} height
         */
        const acceptMerkleBlock = (height) => {
          if (rejected) {
            throw new Error('Unable to accept rejected merkle block');
          }

          if (height < fromBlockHeight) {
            const error = new Error(`Merkle block height is lesser than expected startBlockHeight: ${height} < ${fromBlockHeight}`);
            stream.destroy(error);
            return;
          }

          lastSyncedBlockHeight = height;
          if (addressesGenerated.length) {
            restartArgs = {
              fromBlockHeight: height + 1,
              addresses: [...addresses, ...addressesGenerated],
            };

            addressesGenerated = [];

            // Restart stream to expand bloom filter
            stream.cancel();
          }

          accepted = true;
        };

        const rejectMerkleBlock = (e) => {
          if (accepted) {
            throw new Error('Unable to reject accepted merkle block');
          }
          rejected = true;
          stream.destroy(e);
        };

        this.emit(EVENTS.MERKLE_BLOCK, { merkleBlock, acceptMerkleBlock, rejectMerkleBlock });
      }
    };

    const errorHandler = (streamError) => {
      if (streamError.code === GrpcErrorCodes.CANCELLED) {
        this.continuousSyncStream = null;
        if (restartArgs) {
          logger.debug('[TransactionsReader] Restarting stream with', {
            fromBlockHeight: restartArgs.fromBlockHeight,
            _addressesCount: restartArgs.addresses.length,
          });
          this.startContinuousSync(
            restartArgs.fromBlockHeight,
            restartArgs.addresses,
          ).then((newStream) => {
            this.continuousSyncStream = newStream;
          }).catch((e) => {
            this.emit(EVENTS.ERROR, e);
          }).finally(() => {
            restartArgs = null;
          });
        }

        logger.debug('[TransactionsReader] Stream canceled on client');

        return;
      }

      this.emit(EVENTS.ERROR, streamError);
    };

    const beforeReconnectHandler = (updateArguments) => {
      if (restartArgs) {
        logger.debug('[TransactionsReader] Stream is going to be restarted, skipping reconnect');
        return;
      }
      // If the lastSyncedBlockHeight was not updated yet, re-sync from the same block
      // otherwise sync from the next block from lastSyncedBlockHeight

      // TODO(spv): fix - either don't wipe addressesGenerated,
      // or alter `addresses` to reflect new generated ones
      // because we have a problem with the addresses gap in case of a
      // reconnect and then consequent acceptMerkleBlock
      const newAddresses = addressesGenerated.length
        ? [...addresses, ...addressesGenerated] : addresses;

      logger.debug('[TransactionsReader] Reconnecting to stream with', {
        fromBlockHeight: lastSyncedBlockHeight,
        _addressesCount: newAddresses.length,
      });
      updateArguments(
        createBloomFilter(newAddresses),
        {
          fromBlockHeight: lastSyncedBlockHeight,
          count: 0,
        },
      );
      addressesGenerated = [];
    };

    const endHandler = () => {
      logger.debug('[TransactionsReader] Continuous sync stream ended');
      this.continuousSyncStream = null;
    };

    stream.on('data', dataHandler);
    stream.on('error', errorHandler);
    stream.on('end', endHandler);
    stream.on('beforeReconnect', beforeReconnectHandler);

    return stream;
  }

  async stopHistoricalSync() {
    if (this.historicalSyncStream) {
      await this.historicalSyncStream.cancel();
      this.emit(EVENTS.STOPPED);
      logger.debug('[TransactionsReader] Stopped historical sync');
    }
  }

  async stopContinuousSync() {
    if (this.continuousSyncStream) {
      await this.continuousSyncStream.cancel();
      this.emit(EVENTS.STOPPED);
      logger.debug('[TransactionsReader] Stopped continuous sync');
    }
  }
}

TransactionsReader.EVENTS = EVENTS;

module.exports = TransactionsReader;
