/**
 * This module syncs the header chain
 * No need to use this module manually - it's part of HeaderChainSync.
 * @module HeaderChainProvider
 */
const { SpvChain } = require('@dashevo/dash-spv');
const range = require('lodash/range');

const HeaderChainChunk = require('./HeaderChainChunk');

/**
 * This class syncs the header chain in a parallel manner across masternodes
 */
class HeaderChainProvider {
  /**
   * @param {DAPIClient} api
   * @param {int} mnListLength
   * @param {object} options
   * @param {string} [options.network="mainnet"]
   */
  constructor(api, mnListLength, { network = 'mainnet' }) {
    this.api = api;
    this.mnListLength = mnListLength;
    this.network = network;
  }

  /**
   * @private
   *
   * Retrieve headers for a slice and populate header chain
   *
   * @param {SpvChain} headerChain
   * @param {HeaderChainChunk} headerChainChunk
   * @param {int} [retryCount = 5]
   *
   * @returns {Promise<void>}
   */
  async populateHeaderChain(
    headerChain, headerChainChunk, retryCount = 5,
  ) {
    const addHeadersPromises = range(
      headerChainChunk.getFromHeight(),
      headerChainChunk.getToHeight(),
      headerChainChunk.getStep(),
    )
      .map(async (height) => {
        const newHeaders = await this.api.getBlockHeaders(height, headerChainChunk.getStep());

        let extraHeaders;
        if (headerChainChunk.getExtraSize() > 0) {
          extraHeaders = await this.api.getBlockHeaders(
            height + headerChainChunk.getStep(),
            headerChainChunk.getExtraSize(),
          );
        }

        try {
          headerChain.addHeaders(newHeaders);

          if (extraHeaders) {
            headerChain.addHeaders(extraHeaders);
          }
        } catch (e) {
          if (retryCount > 0) {
            await this.populateHeaderChain(
              headerChain, headerChainChunk, retryCount - 1,
            );
          }
        }
      });

    await Promise.all(addHeadersPromises);
  }

  /**
   * @private
   *
   * Build the header chain for a specified slice
   *
   * @param {int} fromHeight

   * @return {Promise<SpvChain>}
   */
  async buildHeaderChain(fromHeight) {
    const fromBlockHash = await this.api.getBlockHash(fromHeight);
    const fromBlockHeader = await this.api.getBlockHeader(fromBlockHash);
    const toHeight = await this.api.getBestBlockHeight();

    const numConfirms = 10000;

    const headerChain = new SpvChain(this.network, numConfirms, fromBlockHeader);

    const heightDiff = toHeight - fromHeight;

    const chunkSize = Math.floor(heightDiff / this.mnListLength);
    const step = Math.min(chunkSize, 2000);

    /**
     * Naive worker-like implementation of a parallel calls
     *
     *    node1    node2     node3
     *   /    \   /    \   /       \
     *  |  |  |  |  |  |  |  |  |  |
     *  1  2  3  1  2  3  1  2  3  4
     * [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] - header chain
     *
     */

    const promises = range(this.mnListLength).map(async (index) => {
      const chunkStartingHeight = fromHeight + (chunkSize * index);

      // Ask last node a few extra headers
      const heightExtra = (index === this.mnListLength - 1)
        ? heightDiff % Math.min(this.mnListLength, step) : 0;

      const headerChainChunk = new HeaderChainChunk(
        chunkStartingHeight,
        chunkSize + heightExtra, // last node will download a few more headers
        step,
      );

      await this.populateHeaderChain(headerChain, headerChainChunk);
    });

    await Promise.all(promises);

    return headerChain;
  }

  /**
   * Returns simplified header chain from lastChainTipHeight to current chain tip
   * @param {number} lastChainTipHeight - height of the last header stored
   * @returns {Promise<Array<Object>>}
   */
  async fetch(lastChainTipHeight) {
    const headerChain = await this.buildHeaderChain(lastChainTipHeight);

    return headerChain.getLongestChain();
  }
}

module.exports = HeaderChainProvider;
