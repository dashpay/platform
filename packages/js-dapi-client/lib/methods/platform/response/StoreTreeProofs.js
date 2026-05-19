class StoreTreeProofs {
  /**
   * @param {object} properties
   * @param {Uint8Array} properties.publicKeyHashesToIdentityIdsProof
   * @param {Uint8Array} properties.identitiesProof
   * @param {Uint8Array} properties.documentsProof
   * @param {Uint8Array} properties.dataContractsProof
   */
  constructor(properties) {
    this.publicKeyHashesToIdentityIdsProof = properties.publicKeyHashesToIdentityIdsProof;
    this.identitiesProof = properties.identitiesProof;
    this.documentsProof = properties.documentsProof;
    this.dataContractsProof = properties.dataContractsProof;
  }

  /**
   * @returns {Uint8Array}
   */
  getPublicKeyHashesToIdentityIdsProof() {
    return this.publicKeyHashesToIdentityIdsProof;
  }

  /**
   * @returns {Uint8Array}
   */
  getIdentitiesProof() {
    return this.identitiesProof;
  }

  /**
   * @returns {Uint8Array}
   */
  getDocumentsProof() {
    return this.documentsProof;
  }

  /**
   * @returns {Uint8Array}
   */
  getDataContractsProof() {
    return this.dataContractsProof;
  }
}

export default StoreTreeProofs;
