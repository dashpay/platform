const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityKeysResponse extends AbstractResponse {
  /**
   * @param {number} identityKeys
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identityKeys, metadata, proof = undefined) {
    super(metadata, proof);

    this.identityKeys = identityKeys;
  }

  /**
   * @returns {number}
   */
  getIdentityKeys() {
    return this.identityKeys;
  }

  /**
   * @param proto
   * @returns {GetIdentityKeysResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const keys = proto.getV0().getKeys();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    let identityKeys = [];
    if ((typeof keys === 'undefined' || keys === null) && !proof) {
      throw new InvalidResponseError('Identity keys are not defined');
    } else if (!proof) {
      identityKeys = keys.getKeysBytesList();
      if ((typeof identityKeys === 'undefined' || identityKeys === null) && !proof) {
        throw new InvalidResponseError('Identity keys are not defined');
      }
    }

    return new GetIdentityKeysResponse(
      identityKeys,
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentityKeysResponse;
