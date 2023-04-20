const DriveError = require('../../errors/DriveError');

class QuorumsNotFoundError extends DriveError {
  /**
   * @param {SimplifiedMNList} simplifiedMNList
   * @param {number} quorumType
   */
  constructor(simplifiedMNList, quorumType) {
    let message;
    if (simplifiedMNList.quorumList.length === 0) {
      message = `SML at block ${simplifiedMNList.blockHash} contains no quorums of any type`;
    } else {
      const otherQuorumTypes = [...new Set(simplifiedMNList.quorumList.map((quorumEntry) => quorumEntry.llmqType))].join(',');
      message = `SML at block ${simplifiedMNList.blockHash} contains no quorums of type ${quorumType}, but contains entries for types ${otherQuorumTypes}. Please check the Drive configuration`;
    }
    super(message);

    this.simplifiedMNList = simplifiedMNList;
  }

  /**
   * Get block height
   *
   * @return {SimplifiedMNList}
   */
  getSimplifiedMNList() {
    return this.simplifiedMNList;
  }
}

module.exports = QuorumsNotFoundError;
