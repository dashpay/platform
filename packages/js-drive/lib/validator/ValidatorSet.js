const Validator = require('./Validator');
const ValidatorSetIsNotInitializedError = require('./errors/ValidatorSetIsNotInitializedError');
const ValidatorNetworkInfo = require('./ValidatorNetworkInfo');
const QuorumsNotFoundError = require('../core/errors/QuorumsNotFoundError');

class ValidatorSet {
  /**
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {getRandomQuorum} getRandomQuorum
   * @param {fetchQuorumMembers} fetchQuorumMembers
   * @param {number} validatorSetLLMQType
   * @param {RpcClient} coreRpcClient
   * @param {number} tenderdashP2pPort
   */
  constructor(
    simplifiedMasternodeList,
    getRandomQuorum,
    fetchQuorumMembers,
    validatorSetLLMQType,
    coreRpcClient,
    tenderdashP2pPort,
  ) {
    this.simplifiedMasternodeList = simplifiedMasternodeList;
    this.getRandomQuorum = getRandomQuorum;
    this.fetchQuorumMembers = fetchQuorumMembers;
    this.validatorSetLLMQType = validatorSetLLMQType;
    this.coreRpcClient = coreRpcClient;
    this.tenderdashP2pPort = tenderdashP2pPort;

    this.quorum = null;
    this.validators = [];
  }

  /**
   * Chooses an active validator set from among all active validator quorums for the first time
   *
   * @param {number} coreHeight
   */
  async initialize(coreHeight) {
    const sml = this.simplifiedMasternodeList.getStore().getSMLbyHeight(coreHeight);

    // using the block hash at the first core height as entropy
    const rotationEntropy = Buffer.from(sml.toSimplifiedMNListDiff().blockHash, 'hex');

    await this.switchToRandomQuorum(
      sml,
      coreHeight,
      rotationEntropy,
    );
  }

  /**
   * Rotates to a new active validator set from among all active validator quorums
   *
   * @param {Long} height
   * @param {number} coreHeight
   * @param {Buffer} rotationEntropy
   */
  async rotate(height, coreHeight, rotationEntropy) {
    const sml = this.simplifiedMasternodeList.getStore().getSMLbyHeight(coreHeight);

    // validator set is rotated every ROTATION_BLOCK_INTERVAL blocks
    if (height.toNumber() % ValidatorSet.ROTATION_BLOCK_INTERVAL !== 0) {
      return false;
    }

    await this.switchToRandomQuorum(
      sml,
      coreHeight,
      rotationEntropy,
    );

    return true;
  }

  /**
   * Get Validator Set Quorum
   *
   * @return {QuorumEntry}
   */
  getQuorum() {
    if (!this.quorum) {
      throw new ValidatorSetIsNotInitializedError();
    }

    return this.quorum;
  }

  /**
   * Get validators
   *
   * @return {Validator[]}
   */
  getValidators() {
    if (this.validators.length === 0) {
      throw new ValidatorSetIsNotInitializedError();
    }

    return this.validators;
  }

  /**
   * @private
   * @param {SimplifiedMNList} sml
   * @param {number} coreHeight
   * @param {Buffer} rotationEntropy
   * @return {Promise<void>}
   */
  async switchToRandomQuorum(sml, coreHeight, rotationEntropy) {
    const result = await this.getRandomQuorum(
      sml,
      this.validatorSetLLMQType,
      rotationEntropy,
      coreHeight,
    );

    // Stay with the current quorum if there are no any other valid
    if (!result) {
      if (!this.quorum) {
        throw new QuorumsNotFoundError(sml, this.validatorSetLLMQType);
      }

      return;
    }

    const { quorum, members } = result;

    this.quorum = quorum;

    // If the node is a quorum member and doesn't receive public key share for members
    // it should throw an error
    let proTxHash;

    try {
      ({
        result: {
          proTxHash,
        },
      } = await this.coreRpcClient.masternode('status'));
    } catch (e) {
      // This node is not a masternode
      if (e.code !== -32603) {
        throw e;
      }
    }

    const isThisNodeMember = !!members
      .find((member) => member.valid && member.proTxHash === proTxHash);

    this.validators = members.map((member) => {
      const ip = member.service.split(':')[0];

      const networkInfo = new ValidatorNetworkInfo(ip, this.tenderdashP2pPort);

      return Validator.createFromQuorumMember(member, networkInfo, isThisNodeMember);
    });
  }
}

ValidatorSet.ROTATION_BLOCK_INTERVAL = 15;

module.exports = ValidatorSet;
