const cbor = require('cbor');
const Long = require('long');

const ChainInfo = require('./ChainInfo');

class ChainInfoCommonStoreRepository {
  /**
   *
   * @param {CommonStore} commonStore
   */
  constructor(commonStore) {
    this.storage = commonStore;
  }

  /**
   * Store chain info
   *
   * @param {ChainInfo} chainInfo
   * @return {this}
   */
  async store(chainInfo) {
    await this.storage.put(
      ChainInfoCommonStoreRepository.COMMON_STORE_KEY_NAME,
      cbor.encodeCanonical(chainInfo.toJSON()),
    );

    return this;
  }

  /**
   * Fetch chain info
   *
   * @return {ChainInfo}
   */
  async fetch() {
    const chainInfoEncoded = await this.storage.get(
      ChainInfoCommonStoreRepository.COMMON_STORE_KEY_NAME,
    );

    if (!chainInfoEncoded) {
      return new ChainInfo();
    }

    const {
      lastBlockHeight,
      lastBlockAppHash,
    } = cbor.decode(chainInfoEncoded);

    return new ChainInfo(
      Long.fromString(lastBlockHeight),
      lastBlockAppHash,
    );
  }
}

ChainInfoCommonStoreRepository.COMMON_STORE_KEY_NAME = Buffer.from('chainInfo');

module.exports = ChainInfoCommonStoreRepository;
