const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');
const createProofFromRawProof = require('../response/createProofFromRawProof');

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
    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    const rawProof = proto.getProof();

    let proof;
    if (rawProof) {
      proof = createProofFromRawProof(rawProof);
    }

    return new GetIdentitiesByPublicKeyHashesResponse(
      proto.getIdentitiesList()
        .map((identity) => (identity.length > 0 ? Buffer.from(identity) : null)),
      new Metadata(metadata.toObject()),
      proof,
    );
  }
}

module.exports = GetIdentitiesByPublicKeyHashesResponse;
