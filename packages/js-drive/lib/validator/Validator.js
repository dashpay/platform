class Validator {
  /**
   * @param {Buffer} proTxHash
   * @param {Buffer} pubKeyShare
   */
  constructor(proTxHash, pubKeyShare) {
    this.proTxHash = proTxHash;
    this.pubKeyShare = pubKeyShare;
  }

  /**
   * Get validator pro tx hash
   *
   * @return {Buffer}
   */
  getProTxHash() {
    return this.proTxHash;
  }

  /**
   * Get validator public key share
   * @return {Buffer}
   */
  getPublicKeyShare() {
    return this.pubKeyShare;
  }

  /**
   * Get validator voting power
   *
   * @return {number}
   */
  getVotingPower() {
    return Validator.DEFAULT_DASH_VOTING_POWER;
  }

  /**
   * @param {Object} member
   * @return {Validator}
   */
  static createFromQuorumMember(member) {
    const proTxHash = Buffer.from(member.proTxHash, 'hex');

    let pubKeyShare = Buffer.alloc(96);
    if (member.pubKeyShare) {
      pubKeyShare = Buffer.from(member.pubKeyShare, 'hex');
    }

    return new Validator(proTxHash, pubKeyShare);
  }
}

Validator.DEFAULT_DASH_VOTING_POWER = 100;

module.exports = Validator;
