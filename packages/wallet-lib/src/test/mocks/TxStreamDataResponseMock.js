class TxStreamDataResponseMock {
  /**
     *
     * @param options
     * @param {Buffer} [options.rawMerkleBlock]
     * @param {Buffer[]} [options.rawTransactions]
     */
  constructor({ rawMerkleBlock, rawTransactions }) {
    this.rawMerkleBlock = rawMerkleBlock;
    this.rawTransactions = rawTransactions;
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
    return {
      getTransactionsList() {
        return rawTransactions || [];
      },
    };
  }
}

module.exports = TxStreamDataResponseMock;
