class NoDPNSContractFoundError extends Error {
  /**
   * @param {Identifier} contractId
   * @param {number} height
   */
  constructor(contractId, height) {
    super(`DPNS contract with ID ${contractId} on height ${height} was not found`);

    this.name = this.constructor.name;
    this.contractId = contractId;
    this.height = height;

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Get contract id
   *
   * @return {Identifier}
   */
  getContractId() {
    return this.contractId;
  }

  /**
   * Get height
   *
   * @return {number}
   */
  getHeight() {
    return this.height;
  }
}

module.exports = NoDPNSContractFoundError;
