class TxStreamDataResponseMock {
  /**
     *
     * @param options
     * @param {Buffer} [options.rawMerkleBlock]
     * @param {Buffer[]} [options.rawTransactions]
     */
  constructor({ rawMerkleBlock, rawTransactions, instantSendLockMessages }) {
    this.rawMerkleBlock = rawMerkleBlock;
    this.rawTransactions = rawTransactions;
    this.instantSendLockMessages = instantSendLockMessages;
  }

  /**
     * @return {Buffer}
     */
  getRawMerkleBlock() {
    return this.rawMerkleBlock;
  }

  /**
     * @return {{getTransactionsList: (): Buffer[]}}
     */
  getRawTransactions() {
    const { rawTransactions } = this;
    if (rawTransactions) {
      return {
        getTransactionsList() {
          return rawTransactions || [];
        },
      };
    }
    return null;
  }

  getInstantSendLockMessages() {
    const { instantSendLockMessages } = this;
    return {
      getMessagesList() {
        return instantSendLockMessages || [];
      },
    };
  }
}

module.exports = TxStreamDataResponseMock;
