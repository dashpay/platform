const AbstractResponse = require('../response/AbstractResponse');

class GetIdentityByPublicKeyHashResponse extends AbstractResponse {
  /**
   * @param {Uint8Array} identities
   * @param identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identity, metadata, proof = undefined) {
    super(metadata, proof);

    this.identity = identity;
  }

  /**
   * @returns {Uint8Array[]}
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
      new Uint8Array(identity),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityByPublicKeyHashResponse;
