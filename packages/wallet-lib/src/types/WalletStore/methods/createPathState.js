const logger = require('../../../logger');

function createPathState(path) {
  logger.debug(`WalletStore - Creating path state ${path}`);
  if (!this.state.paths.has(path)) {
    this.state.paths.set(path, {
      path,
      addresses: {},
    });
  }
}
module.exports = createPathState;
