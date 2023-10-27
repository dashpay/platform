const AbstractResponse = require('../response/AbstractResponse');

class GetIdentitiesByPublicKeyHashesResponse extends AbstractResponse {
  /**
   * @param {Buffer[]} identities
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identities, metadata, proof = undefined) {
    super(metadata, proof);

    this.identities = identities;
  }

  /**
   * @returns {Buffer[]}
   */
  getIdentities() {
    return this.identities;
  }

  /**
   * @param proto
   * @returns {GetIdentitiesByPublicKeyHashesResponse}
   */
  static createFromProto(proto) {
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    const identitiesList = proto.getV0().getIdentities();

    return new GetIdentitiesByPublicKeyHashesResponse(
      identitiesList !== undefined
        ? identitiesList.getIdentityEntriesList()
          .map((identity) => {
            // TODO: rework to return whole `identity.getValue()` instead of inner value
            const value = identity.getValue();
            return value && value.getValue();
          })
        : [],
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentitiesByPublicKeyHashesResponse;
