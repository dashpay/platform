const cbor = require('cbor');
const Long = require('long');

const ChainInfo = require('./ChainInfo');

class ChainInfoExternalStoreRepository {
  /**
   *
   * @param {LevelUP} externalLevelDB
   */
  constructor(externalLevelDB) {
    this.storage = externalLevelDB;
  }

  /**
   * Store chain info
   *
   * @param {ChainInfo} chainInfo
   * @return {this}
   */
  async store(chainInfo) {
    await this.storage.put(
      ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
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
    try {
      const chainInfoEncoded = await this.storage.get(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      );

      if (!chainInfoEncoded) {
        return new ChainInfo();
      }

      const {
        lastBlockHeight,
      } = cbor.decode(chainInfoEncoded);

      return new ChainInfo(
        Long.fromString(lastBlockHeight),
      );
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return new ChainInfo();
      }

      throw e;
    }
  }
}

ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME = Buffer.from('chainInfo');

module.exports = ChainInfoExternalStoreRepository;
