import InvalidResponseError from '../../platform/response/errors/InvalidResponseError.js';

class GetTransactionResponse {
  /**
   *
   * @param {object} properties
   * @param {Uint8Array} properties.transaction
   * @param {Uint8Array} properties.blockHash
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
   * @returns {Uint8Array}
   */
  getTransaction() {
    return this.transaction;
  }

  /**
   * Get block hash
   * @returns {Uint8Array}
   */
  getBlockHash() {
    return this.blockHash;
  }

  /**
   * Get height
   * @returns {number}
   */
  getHeight() {
    return this.height;
  }

  /**
   * Get number of confirmations
   * @returns {number}
   */
  getConfirmations() {
    return this.confirmations;
  }

  /**
   * Is transaction instant locked
   * @returns {boolean}
   */
  isInstantLocked() {
    return this.instantLocked;
  }

  /**
   * Is transaction chain locked
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

    return new GetTransactionResponse({
      transaction: new Uint8Array(transactionBinaryArray),
      blockHash: new Uint8Array(proto.getBlockHash()),
      height: proto.getHeight(),
      confirmations: proto.getConfirmations(),
      isInstantLocked: proto.getIsInstantLocked(),
      isChainLocked: proto.getIsChainLocked(),
    });
  }
}

export default GetTransactionResponse;
