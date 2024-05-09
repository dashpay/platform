const AbstractResponse = require('../response/AbstractResponse');

class GetIdentityByPublicKeyHashResponse extends AbstractResponse {
  /**
   * @param {Buffer} identities
   * @param identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identity, metadata, proof = undefined) {
    super(metadata, proof);

    this.identity = identity;
  }

  /**
   * @returns {Buffer[]}
   */
  getIdentity() {
    return this.identity;
  }

  /**
   * @param proto
   * @returns {GetIdentityByPublicKeyHashResponse}
   */
  static createFromProto(proto) {
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    const identity = proto.getV0().getIdentity();

    return new GetIdentityByPublicKeyHashResponse(
      Buffer.from(identity),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityByPublicKeyHashResponse;
