const lodashSet = require('lodash.set');
const lodashGet = require('lodash.get');

const systemConfigs = require('./systemConfigs/systemConfigs');
const NETWORKS = require('../networks');

module.exports = {
  '0.17.2': (name, options) => {
    if (options.network !== NETWORKS.TESTNET) {
      return options;
    }

    // Set seed nodes for testnet tenderdash
    lodashSet(options, 'platform.drive.tenderdash.p2p.seeds', systemConfigs.testnet.platform.drive.tenderdash.p2p.seeds);
    lodashSet(options, 'platform.drive.tenderdash.p2p.persistentPeers', []);

    return options;
  },
  '0.17.3': (name, options) => {
    if (options.network !== NETWORKS.TESTNET) {
      return options;
    }

    // Set DashPay contract ID and block height for testnet
    lodashSet(options, 'platform.dashpay', systemConfigs.testnet.platform.dashpay);

    return options;
  },
  '0.17.4': (name, options) => {
    let baseConfig = systemConfigs.base;
    if (systemConfigs[name]) {
      baseConfig = systemConfigs[name];
    }

    const previousStdoutLogLevel = lodashGet(
      options,
      'platform.drive.abci.log.level',
    );

    // Set Drive's new logging variables
    lodashSet(options, 'platform.drive.abci.log', baseConfig.platform.drive.abci.log);

    // Keep previous log level for stdout
    if (previousStdoutLogLevel) {
      lodashSet(options, 'platform.drive.abci.log.stdout.level', previousStdoutLogLevel);
    }
  },
  '0.18.0': (name, options) => {
    lodashSet(options, 'core.sentinel', systemConfigs.base.core.sentinel);

    lodashSet(
      options,
      'platform.drive.tenderdash.docker.image',
      systemConfigs.base.platform.drive.tenderdash.docker.image,
    );

    return options;
  },
};
