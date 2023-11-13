import {AbstractError} from "../../errors/AbstractError.js";

export class GroupIsNotPresentError extends AbstractError {
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
