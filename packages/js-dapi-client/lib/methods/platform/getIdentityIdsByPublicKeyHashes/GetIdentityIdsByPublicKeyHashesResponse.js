const AbstractResponse = require('../response/AbstractResponse');

class GetIdentityIdsByPublicKeyHashesResponse extends AbstractResponse {
  /**
   * @param {Buffer[]} identityIds
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identityIds, metadata, proof = undefined) {
    super(metadata, proof);

    this.identityIds = identityIds;
  }

  /**
   * @returns {Buffer[]}
   */
  getIdentityIds() {
    return this.identityIds;
  }

  /**
   * @param proto
   * @returns {GetIdentityIdsByPublicKeyHashesResponse}
   */
  static createFromProto(proto) {
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    return new GetIdentityIdsByPublicKeyHashesResponse(
      proto.getIdentityIdsList()
        .map((identityId) => (identityId.length > 0 ? Buffer.from(identityId) : null)),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityIdsByPublicKeyHashesResponse;
