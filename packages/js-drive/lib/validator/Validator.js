const PublicKeyShareIsNotPresentError = require('./errors/PublicKeyShareIsNotPresentError');

class Validator {
  /**
   * @param {Buffer} proTxHash
   * @param {ValidatorNetworkInfo} networkInfo
   * @param {Buffer} [pubKeyShare]
   */
  constructor(proTxHash, networkInfo, pubKeyShare = undefined) {
    this.proTxHash = proTxHash;
    this.networkInfo = networkInfo;
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
   * Get network info
   *
   * @returns {ValidatorNetworkInfo}
   */
  getNetworkInfo() {
    return this.networkInfo;
  }

  /**
   * @param {Object} member
   * @param {ValidatorNetworkInfo} networkInfo
   * @param {boolean} [pubKeyShareRequired=false]
   * @return {Validator}
   */
  static createFromQuorumMember(member, networkInfo, pubKeyShareRequired = false) {
    const proTxHash = Buffer.from(member.proTxHash, 'hex');

    let pubKeyShare;
    if (member.pubKeyShare) {
      pubKeyShare = Buffer.from(member.pubKeyShare, 'hex');
    } else if (pubKeyShareRequired) {
      throw new PublicKeyShareIsNotPresentError(member);
    }

    return new Validator(proTxHash, networkInfo, pubKeyShare);
  }
}

Validator.DEFAULT_DASH_VOTING_POWER = 100;

module.exports = Validator;
