/**
 * @param {Config} config
 * @return {boolean}
 */
function isSeedNode(config) {
  return config.getName() === 'local_seed';
}

module.exports = isSeedNode;
