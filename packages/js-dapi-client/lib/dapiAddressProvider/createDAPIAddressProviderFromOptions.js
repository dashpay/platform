const networks = require('@dashevo/dashcore-lib/lib/networks');

const DAPIAddress = require('./DAPIAddress');

const ListDAPIAddressProvider = require('./ListDAPIAddressProvider');

const SimplifiedMasternodeListProvider = require('../SimplifiedMasternodeListProvider/SimplifiedMasternodeListProvider');
const SimplifiedMasternodeListDAPIAddressProvider = require('./SimplifiedMasternodeListDAPIAddressProvider');
const createMasternodeListStreamFactory = require('../SimplifiedMasternodeListProvider/createMasternodeListStreamFactory');

const DAPIClientError = require('../errors/DAPIClientError');

const networkConfigs = require('../networkConfigs');

/**
 * @typedef {createDAPIAddressProviderFromOptions}
 * @param {DAPIClientOptions} options
 * @returns {
 *    DAPIAddressProvider|
 *    ListDAPIAddressProvider|
 *    SimplifiedMasternodeListDAPIAddressProvider|
 *    null
 * }
 */
function createDAPIAddressProviderFromOptions(options) {
  if (options.network && !networks.get(options.network)) {
    throw new DAPIClientError(`Invalid network '${options.network}'`);
  }

  if (options.dapiAddressProvider) {
    if (options.dapiAddresses) {
      throw new DAPIClientError("Can't use 'dapiAddresses' with 'dapiAddressProvider' option");
    }

    if (options.seeds) {
      throw new DAPIClientError("Can't use 'seeds' with 'dapiAddressProvider' option");
    }

    if (options.dapiAddressesWhiteList) {
      throw new DAPIClientError("Can't use 'dapiAddressesWhiteList' with 'dapiAddressProvider' option");
    }

    return options.dapiAddressProvider;
  }

  if (options.dapiAddresses) {
    if (options.seeds) {
      throw new DAPIClientError("Can't use 'seeds' with 'dapiAddresses' option");
    }

    if (options.dapiAddressesWhiteList) {
      throw new DAPIClientError("Can't use 'dapiAddressesWhiteList' with 'dapiAddresses' option");
    }

    return new ListDAPIAddressProvider(
      options.dapiAddresses.map((rawAddress) => new DAPIAddress(rawAddress)),
      options,
    );
  }

  if (options.seeds) {
    let dapiAddressesWhiteList = options.dapiAddressesWhiteList || [];

    // Since we don't have PoSe atm, 3rd party masternodes sometimes provide wrong data
    // that breaks test suite and application logic. Temporary solution is to hardcode
    // reliable DCG testnet masternodes to connect. Should be removed when PoSe is introduced.
    const network = networks.get(options.network);
    let isRegtest = false;
    if (network) {
      isRegtest = network.regtestEnabled;
    }

    if (options.network === 'testnet' && dapiAddressesWhiteList.length === 0 && !isRegtest) {
      dapiAddressesWhiteList = networkConfigs.testnet.dapiAddressesWhiteList;
    }

    const listDAPIAddressProvider = new ListDAPIAddressProvider(
      options.seeds.map((rawAddress) => new DAPIAddress(rawAddress)),
      options,
    );

    const createStream = createMasternodeListStreamFactory(
      createDAPIAddressProviderFromOptions,
      listDAPIAddressProvider,
      options,
    );

    const smlProvider = new SimplifiedMasternodeListProvider(
      createStream,
      options,
    );

    return new SimplifiedMasternodeListDAPIAddressProvider(
      smlProvider,
      listDAPIAddressProvider,
      dapiAddressesWhiteList.map((rawAddress) => new DAPIAddress(rawAddress)),
    );
  }

  if (options.network) {
    if (!networkConfigs[options.network]) {
      throw new DAPIClientError(`There is no connection config for network '${options.network}'`);
    }

    const networkConfig = { ...options, ...networkConfigs[options.network] };

    return createDAPIAddressProviderFromOptions(networkConfig);
  }

  return null;
}

module.exports = createDAPIAddressProviderFromOptions;
