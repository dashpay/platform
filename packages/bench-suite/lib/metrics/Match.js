const lodashMatches = require('lodash/matches');

class Match {
  /**
   * @type {Function}
   */
  #isMatch;

  /**
   * @type {Function}
   */
  #callback;

  /**
   * @param {Object} pattern
   * @param {Function} callback
   */
  constructor(pattern, callback) {
    this.#isMatch = lodashMatches(pattern);
    this.#callback = callback;
  }

  /**
   * Call match callback if data is matched
   *
   * @param {Object} data
   */
  applyMatch(data) {
    if (this.#isMatch(data)) {
      this.#callback(data);
    }
  }
}

module.exports = Match;
