import logger from '../../../logger/index.js';

function createPathState(path) {
  logger.debug(`WalletStore - Creating path state ${path}`);
  if (!this.state.paths.has(path)) {
    this.state.paths.set(path, {
      path,
      addresses: {},
    });
  }
}
export default createPathState;