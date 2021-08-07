const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');
const createProofFromRawProof = require('../response/createProofFromRawProof');

class GetIdentityResponse extends AbstractResponse {
  /**
   * @param {Buffer} identity
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identity, metadata, proof = undefined) {
    super(metadata, proof);

    this.identity = identity;
  }

  /**
   * @returns {Buffer}
   */
  getIdentity() {
    return this.identity;
  }

  /**
   * @param proto
   * @returns {GetIdentityResponse}
   */
  static createFromProto(proto) {
    const identity = proto.getIdentity();
    const rawProof = proto.getProof();

    if (!identity && !rawProof) {
      throw new InvalidResponseError('Identity is not defined');
    }

    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    let proof;
    if (rawProof) {
      proof = createProofFromRawProof(rawProof);
    }

    return new GetIdentityResponse(
      Buffer.from(proto.getIdentity()),
      new Metadata(metadata.toObject()),
      proof,
    );
  }
}

module.exports = GetIdentityResponse;
