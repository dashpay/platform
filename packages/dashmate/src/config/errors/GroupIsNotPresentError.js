const AbstractError = require('../../errors/AbstractError');

class GroupIsNotPresentError extends AbstractError {
  /**
   * @param {string} groupName
   */
  constructor(groupName) {
    super(`Group with name '${groupName}' is not present`);

    this.groupName = groupName;
  }

  /**
   * @returns {string}
   */
  getGroupName() {
    return this.groupName;
  }
}

module.exports = GroupIsNotPresentError;
