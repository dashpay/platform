const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');
const createProofFromRawProof = require('../response/createProofFromRawProof');

class GetDataContractResponse extends AbstractResponse {
  /**
   * @param {Buffer} dataContract
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(dataContract, metadata, proof = undefined) {
    super(metadata, proof);

    this.dataContract = dataContract;
  }

  /**
   * @returns {Buffer}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * @param proto
   * @returns {GetDataContractResponse}
   */
  static createFromProto(proto) {
    const dataContract = proto.getDataContract();
    const rawProof = proto.getProof();

    if (!dataContract && !rawProof) {
      throw new InvalidResponseError('DataContract is not defined');
    }

    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    let proof;
    if (rawProof) {
      proof = createProofFromRawProof(rawProof);
    }

    return new GetDataContractResponse(
      Buffer.from(dataContract),
      new Metadata(metadata.toObject()),
      proof,
    );
  }
}

module.exports = GetDataContractResponse;
