const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');
const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');

const logger = require('../logger');

class SimplifiedMasternodeListProvider {
  /**
   * @param {Function} createStream - JsonRpcTransport instance
   * @param {object} [options] - Options
   * @param {string} [options.network]
   * @param {string} [options.loggerOptions]
   */
  constructor(createStream, options = {}) {
    this.createStream = createStream;
    this.options = options;
    this.logger = logger.getForId(
      this.options.loggerOptions.identifier,
      this.options.loggerOptions.level,
    );

    /**
     * @type {ReconnectableStream}
     */
    this.stream = undefined;
    this.removeStreamListeners = () => {};

    /**
     * @type {SimplifiedMNList}
     */
    this.simplifiedMNList = new SimplifiedMNList(undefined);
  }

  /**
   * Returns simplified masternode list
   * @returns {Promise<SimplifiedMNList>}
   */
  async getSimplifiedMNList() {
    if (this.stream === undefined) {
      await this.subscribeToMasternodeList();
    }

    return this.simplifiedMNList;
  }

  /**
   * Subscribe to simplified masternodes list updates. No need to call it manually
   * @private
   * @returns {Promise<void>}
   */
  async subscribeToMasternodeList() {
    if (this.stream) {
      this.logger.debug('Masternode list stream already started');
      return Promise.resolve();
    }

    this.logger.debug('Starting masternode list stream');

    this.stream = await this.createStream();

    let diffCount = 0;
    let resolved = false;

    const rejectDiff = (error) => {
      this.logger.silly('Stream is cancelled due to error. Retrying...', { error });

      this.stream.cancel();
      this.stream.retryOnError(error);
    };

    return new Promise((resolve, reject) => {
      const errorHandler = (error) => {
        this.stream = null;

        this.logger.error(
          `Masternode list sync failed: ${error.message}`,
          { error, diffCount },
        );

        if (!resolved) {
          reject(error);
          resolved = true;
        }
      };

      const dataHandler = (response) => {
        diffCount += 1;

        if (diffCount === 1) {
          this.logger.silly(
            'Full masternode list diff received',
            { diffCount },
          );
        } else {
          this.logger.silly(
            'Received masternode list diff',
            { diffCount },
          );
        }

        let simplifiedMNListDiff;
        let simplifiedMNListDiffBuffer;
        try {
          simplifiedMNListDiffBuffer = Buffer.from(response.getMasternodeListDiff_asU8());
          simplifiedMNListDiff = new SimplifiedMNListDiff(
            simplifiedMNListDiffBuffer,
            this.options.network,
          );
        } catch (e) {
          this.logger.warn(
            `Can't parse masternode list diff: ${e.message}`,
            {
              diffCount,
              network: this.options.network,
              error: e,
              simplifiedMNListDiff: simplifiedMNListDiffBuffer.toString('hex'),
            },
          );

          rejectDiff(e);

          return;
        }

        this.logger.silly(
          'Parsed masternode list diff successfully',
          {
            diffCount,
            blockHash: simplifiedMNListDiff.blockHash,
          },
        );

        try {
          // Restart list when we receive a full diff
          if (diffCount === 1) {
            this.simplifiedMNList = new SimplifiedMNList(simplifiedMNListDiff);
          } else {
            this.simplifiedMNList.applyDiff(simplifiedMNListDiff);
          }
        } catch (e) {
          this.logger.warn(
            `Can't apply masternode list diff: ${e.message}`,
            {
              diffCount,
              network: this.options.network,
              blockHash: simplifiedMNListDiff.blockHash,
              error: e,
              simplifiedMNListDiff,
            },
          );

          rejectDiff(e);
        }

        this.logger.silly(
          'Masternode list diff applied successfully',
          {
            diffCount,
            blockHash: simplifiedMNListDiff.blockHash,
          },
        );

        if (!resolved) {
          resolve();
          resolved = true;
        }
      };

      const beforeReconnectHandler = () => {
        diffCount = 0;

        this.logger.debug(
          'Restarting masternode list stream',
          { diffCount },
        );
      };

      const endHandler = () => {
        this.logger.warn(
          'Masternode list sync stopped',
          { diffCount },
        );

        this.removeStreamListeners();
        this.stream = null;
      };

      this.stream.on('data', dataHandler);
      this.stream.on('beforeReconnect', beforeReconnectHandler);
      this.stream.on('error', errorHandler);
      this.stream.on('end', endHandler);

      this.removeStreamListeners = () => {
        this.stream.removeListener('data', dataHandler);
        this.stream.removeListener('beforeReconnect', beforeReconnectHandler);
        this.stream.removeListener('error', errorHandler);
        this.stream.removeListener('end', endHandler);
      };
    });
  }

  /**
   * Unsubscribe from masternode list updates
   */
  unsubscribe() {
    if (this.stream) {
      this.removeStreamListeners();
      this.stream.cancel();
      this.stream = null;
    }
  }
}

SimplifiedMasternodeListProvider.NULL_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

module.exports = SimplifiedMasternodeListProvider;
