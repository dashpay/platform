const PublicKeyShareIsNotPresentError = require('./errors/PublicKeyShareIsNotPresentError');

class Validator {
  /**
   * @param {Buffer} proTxHash
   * @param {Buffer} [pubKeyShare]
   */
  constructor(proTxHash, pubKeyShare = undefined) {
    this.proTxHash = proTxHash;
    this.pubKeyShare = pubKeyShare;
    this.networkInfo = null;
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
   * Set network info
   *
   * @param {ValidatorNetworkInfo} networkInfo
   */
  setNetworkInfo(networkInfo) {
    this.networkInfo = networkInfo;
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

    const validator = new Validator(proTxHash, pubKeyShare);

    validator.setNetworkInfo(networkInfo);

    return validator;
  }
}

Validator.DEFAULT_DASH_VOTING_POWER = 100;

module.exports = Validator;
