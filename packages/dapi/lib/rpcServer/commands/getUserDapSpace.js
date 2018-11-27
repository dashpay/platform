const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getUserDapSpace');

const validator = new Validator(argsSchema);
/**
 * @param dashDrive
 * @return {getUserDapSpace}
 */
const getUserDapSpaceFactory = (dashDrive) => {
  /**
   * Returns user dap space
   * @typedef getUserDapSpace
   * @param args - command arguments
   * @param {string} args.dapId
   * @param {string} args.userId
   * @return {Promise<object>}
   */
  async function getUserDapSpace(args) {
    validator.validate(args);
    const { dapId, userId } = args;
    return dashDrive.getDapSpace(dapId, userId);
  }

  return getUserDapSpace;
};

module.exports = getUserDapSpaceFactory;
