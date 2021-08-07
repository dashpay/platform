class StoreTreeProofs {
  /**
   * @param {Object} properties
   * @param {Buffer} properties.publicKeyHashesToIdentityIdsProof
   * @param {Buffer} properties.identitiesProof
   * @param {Buffer} properties.documentsProof
   * @param {Buffer} properties.dataContractsProof
   */
  constructor(properties) {
    this.publicKeyHashesToIdentityIdsProof = properties.publicKeyHashesToIdentityIdsProof;
    this.identitiesProof = properties.identitiesProof;
    this.documentsProof = properties.documentsProof;
    this.dataContractsProof = properties.dataContractsProof;
  }

  /**
   * @returns {Buffer}
   */
  getPublicKeyHashesToIdentityIdsProof() {
    return this.publicKeyHashesToIdentityIdsProof;
  }

  /**
   * @returns {Buffer}
   */
  getIdentitiesProof() {
    return this.identitiesProof;
  }

  /**
   * @returns {Buffer}
   */
  getDocumentsProof() {
    return this.documentsProof;
  }

  /**
   * @returns {Buffer}
   */
  getDataContractsProof() {
    return this.dataContractsProof;
  }
}

module.exports = StoreTreeProofs;
