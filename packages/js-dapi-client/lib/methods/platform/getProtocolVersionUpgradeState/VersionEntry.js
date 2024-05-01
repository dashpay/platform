class VersionEntry {
  /**
   * @param {number} versionNumber
   * @param {number} voteCount
   */
  constructor(versionNumber, voteCount) {
    this.versionNumber = versionNumber;
    this.voteCount = voteCount;
  }

  /**
   * @returns {number}
   */
  getVersionNumber() {
    return this.versionNumber;
  }

  /**
   * @returns {number}
   */
  getVoteCount() {
    return this.voteCount;
  }
}

module.exports = VersionEntry;
