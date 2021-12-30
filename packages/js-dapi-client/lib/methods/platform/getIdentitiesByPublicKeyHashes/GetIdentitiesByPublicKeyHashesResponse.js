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
   * @returns {Array<Buffer[]>}
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

    return new GetIdentitiesByPublicKeyHashesResponse(
      proto.getIdentitiesList().map((identitiesSerialized) => {
        const identities = cbor.decode(identitiesSerialized);

        return identities.map((identity) => (
          (identity.length > 0 ? Buffer.from(identity) : null)
        ));
      }),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentitiesByPublicKeyHashesResponse;
