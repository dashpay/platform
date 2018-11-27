const Validator = require('../../utils/Validator');
// TODO: Add name validation
const argsSchema = require('../schemas/searchUsers');

const validator = new Validator(argsSchema);

/**
 * @param userIndex
 * @return {function({pattern: string, limit: number, offset: number}): {totalCount: *, results: *}}
 */
const searchDapContractsFactory = (userIndex) => {
  /**
   * Layer 2 endpoint
   * @param args
   * @param {string} args.pattern
   * @param {number} args.limit
   * @param {number} args.offset
   * @return {Promise<{totalCount: number, results: Array<string>}>}
   */
  async function searchUsers(args) {
    validator.validate(args);
    const { pattern, limit, offset } = args;
    const usernames = await userIndex.searchUsernames(pattern);
    return {
      totalCount: usernames.length,
      results: usernames.slice(offset, offset + limit),
    };
  }

  return searchUsers;
};

module.exports = searchDapContractsFactory;
