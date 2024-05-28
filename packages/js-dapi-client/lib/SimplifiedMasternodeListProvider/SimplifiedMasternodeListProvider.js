const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');
const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');
const {
  v0: {
    MasternodeListRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const logger = require('../logger');

class SimplifiedMasternodeListProvider {
  /**
   * @param {GrpcTransport} grpcTransport - JsonRpcTransport instance
   * @param {object} [options] - Options
   * @param {string} [options.network]
   * @param {string} [options.loggerOptions]
   */
  constructor(grpcTransport, options = {}) {
    this.grpcTransport = grpcTransport;
    this.options = options;
    this.logger = logger.getForId(
      this.options.loggerOptions.identifier,
      this.options.loggerOptions.level,
    );

    this.stream = undefined;
    this.simplifiedMNList = undefined;
  }

  /**
   * Returns simplified masternode list
   * @returns {Promise<SimplifiedMNList>}
   */
  async getSimplifiedMNList() {
    if (this.simplifiedMNList === undefined) {
      this.simplifiedMNList = new SimplifiedMNList(undefined, this.options.network);

      await this.subscribeToMasternodeList(0);
    }

    return this.simplifiedMNList;
  }

  /**
   * Subscribe to simplified masternodes list updates. No need to call it manually
   * @private
   * @returns {Promise<void>}
   */
  async subscribeToMasternodeList(sessionId) {
    this.logger.debug(
      'Starting masternode list stream',
      { session: sessionId },
    );

    this.stream = await this.grpcTransport.request(
      CorePromiseClient,
      'subscribeToMasternodeList',
      new MasternodeListRequest(),
      {
        timeout: undefined,
      },
    );

    this.logger.debug(
      'Masternode list stream started',
      { session: sessionId },
    );

    return new Promise((resolve, reject) => {
      let resolved = false;
      let restarted = false;
      let diffCount = 0;

      const restartStream = () => {
        if (restarted) {
          return;
        }

        restarted = true;

        this.stream.cancel();
        this.stream = undefined;
        this.simplifiedMNList = new SimplifiedMNList(undefined);

        this.logger.debug(
          'Restarting masternode list stream',
          { session: sessionId },
        );

        // Start new session in 1 second
        setTimeout(() => {
          this.subscribeToMasternodeList(sessionId + 1).then(() => {
            if (!resolved) {
              resolve();
              resolved = true;
            }
          }).catch((e) => {
            if (!resolved) {
              reject(e);
              resolved = true;
            }
          });
        }, 1000);
      };

      this.stream.on('error', (error) => {
        if (restarted) {
          return;
        }

        this.logger.warn(
          `Masternode list sync failed: ${error.message}`,
          { error, session: sessionId },
        );

        restartStream();
      });

      this.stream.on('end', () => {
        if (restarted) {
          return;
        }

        this.logger.warn(
          'Masternode list sync stopped',
          { session: sessionId },
        );

        restartStream();
      });

      this.stream.on('data', async (response) => {
        if (restarted) {
          return;
        }

        diffCount += 1;

        this.logger.debug(
          'Received masternode list diff',
          { session: sessionId, diffCount },
        );

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
              session: sessionId,
              diffCount,
              network: this.options.network,
              error: e,
              simplifiedMNListDiff: simplifiedMNListDiffBuffer.toString('hex'),
            },
          );
          restartStream();
          return;
        }

        this.logger.debug(
          'Parsed masternode list diff',
          {
            session: sessionId,
            diffCount,
            blockHash: simplifiedMNListDiff.blockHash,
          },
        );

        try {
          this.simplifiedMNList.applyDiff(simplifiedMNListDiff);

          this.logger.debug(
            'Masternode list diff applied successfully',
            {
              session: sessionId,
              diffCount,
              blockHash: simplifiedMNListDiff.blockHash,
            },
          );

          if (!resolved) {
            resolve();
            resolved = true;
          }
        } catch (e) {
          this.logger.warn(
            `Can't apply masternode list diff: ${e.message}`,
            {
              session: sessionId,
              diffCount,
              network: this.options.network,
              blockHash: simplifiedMNListDiff.blockHash,
              error: e,
              simplifiedMNListDiff,
            },
          );

          restartStream();
        }
      });
    });
  }
}

SimplifiedMasternodeListProvider.NULL_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

module.exports = SimplifiedMasternodeListProvider;
