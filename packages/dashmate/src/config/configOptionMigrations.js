const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const systemConfigs = require('./systemConfigs/systemConfigs');

module.exports = {
  '0.17.0-dev.12': (name, options) => {
    // Rename tendermint to tenderdash
    lodashSet(options, 'platform.drive.tenderdash', options.platform.drive.tendermint);
    // eslint-disable-next-line no-param-reassign
    delete options.platform.drive.tendermint;

    // Copy new configs from system defaults
    const sourceConfigName = name in systemConfigs ? name : 'base';
    const paths = [
      'platform.dapi.nginx.rateLimiter.enable',
      'platform.dapi.nginx.rateLimiter.burst',
      'platform.dapi.nginx.rateLimiter.rate',
      'platform.drive.tenderdash.validatorKey',
      'platform.drive.tenderdash.nodeKey',
    ];

    paths.forEach((path) => {
      lodashSet(options, path, lodashGet(systemConfigs[sourceConfigName], path));
    });

    return options;
  },
};
