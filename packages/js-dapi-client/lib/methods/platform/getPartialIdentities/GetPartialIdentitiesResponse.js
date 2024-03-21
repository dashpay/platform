const AbstractResponse = require('../response/AbstractResponse');

class GetPartialIdentitiesResponse extends AbstractResponse {
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
   * @returns {GetPartialIdentitiesResponse}
   */
  static createFromProto(proto) {
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    const identitiesList = proto.getV0().getIdentities();

    return new GetPartialIdentitiesResponse(
      identitiesList !== undefined
        ? identitiesList.getIdentityEntriesList()
          .map((identity) => {
            const value = identity.getValue();
            // TODO: rework to return whole `identity.getValue()` instead of inner getValue()
            return value && Buffer.from(value.getValue());
          })
        : [],
      metadata,
      proof,
    );
  }
}

module.exports = GetPartialIdentitiesResponse;
