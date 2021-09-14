const InvalidResponseError = require('../../platform/response/errors/InvalidResponseError');

class GetTransactionResponse {
  /**
   *
   * @param {object} properties
   * @param {Buffer} properties.transaction
   * @param {Buffer} properties.blockHash
   * @param {number} properties.height
   * @param {number} properties.confirmations
   * @param {boolean} properties.isInstantLocked
   * @param {boolean} properties.isChainLocked
   */
  constructor(properties) {
    this.transaction = properties.transaction;
    this.blockHash = properties.blockHash;
    this.height = properties.height;
    this.confirmations = properties.confirmations;
    this.instantLocked = properties.isInstantLocked;
    this.chainLocked = properties.isChainLocked;
  }

  /**
   * Get transaction
   *
   * @returns {Buffer}
   */
  getTransaction() {
    return this.transaction;
  }

  /**
   * Get block hash
   *
   * @returns {Buffer}
   */
  getBlockHash() {
    return this.blockHash;
  }

  /**
   * Get height
   *
   * @returns {number}
   */
  getHeight() {
    return this.height;
  }

  /**
   * Get number of confirmations
   *
   * @returns {number}
   */
  getConfirmations() {
    return this.confirmations;
  }

  /**
   * Is transaction instant locked
   *
   * @returns {boolean}
   */
  isInstantLocked() {
    return this.instantLocked;
  }

  /**
   * Is transaction chain locked
   *
   * @returns {boolean}
   */
  isChainLocked() {
    return this.chainLocked;
  }

  static createFromProto(proto) {
    const transactionBinaryArray = proto.getTransaction();
    if (!transactionBinaryArray) {
      throw new InvalidResponseError('Transaction is not defined');
    }

    const blockHashBinaryArray = proto.getBlockHash();
    if (!blockHashBinaryArray) {
      throw new InvalidResponseError('BlockHash is not defined');
    }

    return new GetTransactionResponse({
      transaction: Buffer.from(transactionBinaryArray),
      blockHash: Buffer.from(blockHashBinaryArray),
      height: proto.getHeight(),
      confirmations: proto.getConfirmations(),
      isInstantLocked: proto.getIsInstantLocked(),
      isChainLocked: proto.getIsChainLocked(),
    });
  }
}

module.exports = GetTransactionResponse;
