const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetContestedResourceVoteStateResponse extends AbstractResponse {
  /**
   * @param {object} contestedResourceContenders
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(contestedResourceContenders, metadata, proof = undefined) {
    super(metadata, proof);

    this.contestedResourceContenders = contestedResourceContenders;
  }

  /**
   * @returns {object}
   */
  getContestedResourceContenders() {
    return this.contestedResourceContenders;
  }

  /**
   * @param proto
   * @returns {GetContestedResourceVoteStateResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const contestedResourceContenders = proto.getV0().getContestedResourceContenders();

    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof contestedResourceContenders === 'undefined' || contestedResourceContenders === null) && !proof) {
      throw new InvalidResponseError('Contested Resource Contenders data is not defined');
    }

    return new GetContestedResourceVoteStateResponse(
      contestedResourceContenders.toObject(),
      metadata,
      proof,
    );
  }
}

module.exports = GetContestedResourceVoteStateResponse;
