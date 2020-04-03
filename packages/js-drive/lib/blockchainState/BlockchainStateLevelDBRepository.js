const cbor = require('cbor');
const Long = require('long');

const BlockchainState = require('./BlockchainState');

class BlockchainStateLevelDBRepository {
  /**
   *
   * @param {LevelUP} blockchainStateLevelDB
   */
  constructor(blockchainStateLevelDB) {
    this.db = blockchainStateLevelDB;
  }

  /**
   * Store blockchain state
   *
   * @param {BlockchainState} blockchainState
   * @return {this}
   */
  async store(blockchainState) {
    await this.db.put(
      BlockchainStateLevelDBRepository.KEY_NAME,
      cbor.encode(blockchainState.toJSON()),
    );

    return this;
  }

  /**
   * Fetch blockchain state
   *
   * @return {BlockchainState}
   */
  async fetch() {
    try {
      const blockchainStateEncoded = await this.db.get(
        BlockchainStateLevelDBRepository.KEY_NAME,
      );

      const {
        lastBlockHeight,
        lastBlockAppHash,
      } = cbor.decode(blockchainStateEncoded);

      return new BlockchainState(
        Long.fromString(lastBlockHeight),
        lastBlockAppHash,
      );
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return new BlockchainState();
      }

      throw e;
    }
  }
}

BlockchainStateLevelDBRepository.KEY_NAME = 'blockchainState';

module.exports = BlockchainStateLevelDBRepository;
