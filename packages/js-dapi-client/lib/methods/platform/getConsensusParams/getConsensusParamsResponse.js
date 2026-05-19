import InvalidResponseError from '../response/errors/InvalidResponseError.js';
import ConsensusParamsBlock from './ConsensusParamsBlock.js';
import ConsensusParamsEvidence from './ConsensusParamsEvidence.js';

class GetConsensusParamsResponse {
  /**
   *
   * @param {ConsensusParamsBlock} block
   * @param {ConsensusParamsEvidence} evidence
   */
  constructor(block, evidence) {
    this.block = block;
    this.evidence = evidence;
  }

  /**
   * @returns {ConsensusParamsBlock}
   */
  getBlock() {
    return this.block;
  }

  /**
   * @returns {ConsensusParamsEvidence}
   */
  getEvidence() {
    return this.evidence;
  }

  /**
   * @param proto
   * @returns {GetConsensusParamsResponse}
   */
  static createFromProto(proto) {
    const protoBlock = proto.getV0().getBlock();
    const protoEvidence = proto.getV0().getEvidence();

    if (!protoBlock && !protoEvidence) {
      throw new InvalidResponseError('Consensus params are not defined');
    }

    const block = new ConsensusParamsBlock(
      protoBlock.getMaxBytes(),
      protoBlock.getMaxGas(),
      protoBlock.getTimeIotaMs(),
    );

    const evidence = new ConsensusParamsEvidence(
      protoEvidence.getMaxAgeNumBlocks(),
      protoEvidence.getMaxAgeDuration(),
      protoEvidence.getMaxBytes(),
    );

    return new GetConsensusParamsResponse(
      block,
      evidence,
    );
  }
}

export default GetConsensusParamsResponse;
