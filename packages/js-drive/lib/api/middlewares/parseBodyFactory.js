const { utils } = require('jayson');

/**
 * @returns {parseBody}
 */
function parseBodyFactory() {
  /**
   * @typedef {Promise} parseBody
   * @param req
   * @param res
   * @param next
   * @returns {Promise<void>}
   */
  async function parseBody(req, res, next) {
    utils.parseBody(req, null, (err, body) => {
      if (err) {
        next(err);
        return;
      }
      req.body = body;
      next();
    });
  }

  return parseBody;
}

module.exports = parseBodyFactory;
