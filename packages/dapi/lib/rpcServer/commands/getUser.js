const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getUser');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getUser}
 */
const getUserFactory = (coreAPI) => {
  /**
   * Layer 2 endpoint
   * Returns blockchain user
   * @typedef getUser
   * @param args
   * @param {string} args.usernameOrRegTxId
   * @return {Promise<object>}
   */
  async function getUser(args) {
    validator.validate(args);
    const { username, userId } = args;
    return coreAPI.getUser(username || userId);
  }

  return getUser;
};

module.exports = getUserFactory;
