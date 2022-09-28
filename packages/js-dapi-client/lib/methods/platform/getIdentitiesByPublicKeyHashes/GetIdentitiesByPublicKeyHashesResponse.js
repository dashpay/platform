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

    return new GetIdentitiesByPublicKeyHashesResponse(
      proto.getIdentitiesList_asU8().map((identity) => Buffer.from(identity)),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentitiesByPublicKeyHashesResponse;
