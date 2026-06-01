const AbstractResponse = require('../response/AbstractResponse');

class GetIdentityByPublicKeyHashResponse extends AbstractResponse {
  /**
   * @param {Uint8Array} identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identity, metadata, proof = undefined) {
    super(metadata, proof);

    this.identity = identity;
  }

  /**
   * @returns {Uint8Array}
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

    return new GetIdentityByPublicKeyHashResponse(
      // Use _asU8 so we get bytes regardless of the underlying protobuf
      // representation (grpc-js: Uint8Array; grpc-web: base64 string).
      // new Uint8Array(string) does NOT base64-decode, silently losing bytes.
      proto.getV0().getIdentity_asU8(),
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityByPublicKeyHashResponse;
