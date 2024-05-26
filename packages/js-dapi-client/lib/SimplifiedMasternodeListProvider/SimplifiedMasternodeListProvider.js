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
   */
  constructor(grpcTransport, options = {}) {
    this.grpcTransport = grpcTransport;

    this.options = options;
  }

  /**
   * Returns simplified masternode list
   * @returns {Promise<SimplifiedMNList>}
   */
  async getSimplifiedMNList() {
    if (this.simplifiedMNList === undefined) {
      this.simplifiedMNList = new SimplifiedMNList(undefined, this.options.network);

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
    const stream = await this.grpcTransport.request(
      CorePromiseClient,
      'subscribeToTransactionsWithProofs',
      new MasternodeListRequest(),
      {
        retries: Infinity,
      },
    );

    return new Promise((resolve, reject) => {
      let resolved = false;

      const restartStream = () => {
        this.reset();
        stream.removeAllListeners();
        stream.cancel();

        this.subscribeToMasternodeList().then(() => {
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
      };

      stream.on('error', (error) => {
        logger.error(`Masternode list sync failed: ${error.message}. Restarted.`, { error });

        restartStream();
      });

      stream.on('end', () => {
        logger.error('Masternode list sync stopped. Restarted.');

        restartStream();
      });

      stream.on('data', async (response) => {
        const simplifiedMNListDiffBuffer = Buffer.from(response.getMasternodeListDiff_asU8());
        const simplifiedMNListDiff = new SimplifiedMNListDiff(
          simplifiedMNListDiffBuffer,
          this.options.network,
        );

        try {
          this.simplifiedMNList.applyDiff(simplifiedMNListDiff);

          if (!resolved) {
            resolve();
            resolved = true;
          }
        } catch (e) {
          logger.error(`Masternode list sync failed: ${e.message}. Restarted.`, { error: e });

          restartStream();
        }
      });
    });
  }

  /**
   * Reset simplifiedMNList
   * @private
   */
  reset() {
    this.simplifiedMNList = new SimplifiedMNList(undefined, this.options.network);
  }
}

SimplifiedMasternodeListProvider.NULL_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

module.exports = SimplifiedMasternodeListProvider;
