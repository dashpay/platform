const { EventEmitter } = require('events');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const { createBloomFilter, parseRawTransactions, parseRawMerkleBlock } = require('./utils');

const EVENTS = {
  HISTORICAL_TRANSACTIONS: 'HISTORICAL_TRANSACTIONS',
  NEW_TRANSACTIONS: 'NEW_TRANSACTIONS',
  MERKLE_BLOCK: 'MERKLE_BLOCK',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
};

/**
 * @typedef TransactionsReaderOptions
 * @property {Function} [createHistoricalSyncStream]
 * @property {Function} [createContinuousSyncStream]
 * @property {string} network
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
      throw new Error('Historical sync is already in process.');
    }

    if (fromBlockHeight <= 1) {
      throw new Error('Invalid fromBlockHeight');
    }

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount <= 0) {
      throw new Error(`Invalid total amount of blocks to sync: ${totalAmount}`);
    }

    // this.historicalSyncStream = await
  }

  /**
   * @private
   *
   * @param {number} fromBlockHeight
   * @param {number} count
   * @param {string[]} addresses
   * @return Promise<!grpc.web.ClientReadableStream>
   */
  async subscribeToHistoricalStream(fromBlockHeight, count, addresses) {
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
          accepted = true;

          if (newAddresses.length) {
            const blocksRead = height - fromBlockHeight + 1;
            const remainingCount = count - blocksRead;
            if (remainingCount === 0) {
              return;
            }

            if (remainingCount < 0) {
              const error = new Error(`Merkle block height is greater than expected range: ${height} > ${fromBlockHeight + count - 1}`);
              stream.destroy(error);
              return;
            }

            restartArgs = {
              fromBlockHeight: height + 1,
              count: remainingCount,
              addresses: [...addresses, ...newAddresses],
            };

            // Restart stream to expand bloom filter
            stream.cancel()
              .catch((e) => this.emit(EVENTS.ERROR, e));
          }
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
        this.historicalSyncStream = null;
        if (restartArgs) {
          this.subscribeToHistoricalStream(
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

        return;
      }

      this.emit(EVENTS.ERROR, streamError);
    };

    const endHandler = () => {
      if (!restartArgs) {
        this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      }
    };

    stream.on('data', dataHandler);
    stream.on('error', errorHandler);
    stream.on('end', endHandler);

    return stream;
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

          accepted = true;
          lastSyncedBlockHeight = height;
          if (addressesGenerated.length) {
            restartArgs = {
              fromBlockHeight: height + 1,
              addresses: [...addresses, ...addressesGenerated],
            };

            addressesGenerated = [];

            // Restart stream to expand bloom filter
            stream.cancel()
              .catch((e) => this.emit(EVENTS.ERROR, e));
          }
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

        return;
      }

      this.emit(EVENTS.ERROR, streamError);
    };

    const beforeReconnectHandler = (updateArguments) => {
      if (restartArgs) {
        return;
      }
      // If the lastSyncedBlockHeight was not updated yet, re-sync from the same block
      // otherwise sync from the next block from lastSyncedBlockHeight
      const newFromBlockHeight = fromBlockHeight === lastSyncedBlockHeight
        ? fromBlockHeight : lastSyncedBlockHeight + 1;

      const newAddresses = addressesGenerated.length
        ? [...addresses, ...addressesGenerated] : addresses;

      updateArguments(
        createBloomFilter(newAddresses),
        {
          fromBlockHeight: newFromBlockHeight,
          count: 0,
        },
      );
      addressesGenerated = [];
    };

    const endHandler = () => {
      this.continuousSyncStream = null;
    };

    stream.on('data', dataHandler);
    stream.on('error', errorHandler);
    stream.on('end', endHandler);
    stream.on('beforeReconnect', beforeReconnectHandler);

    return stream;
  }

  async stopReadingHistorical() {
    if (this.historicalSyncStream) {
      await this.historicalSyncStream.cancel();
    }
  }
}

TransactionsReader.EVENTS = EVENTS;

module.exports = TransactionsReader;
