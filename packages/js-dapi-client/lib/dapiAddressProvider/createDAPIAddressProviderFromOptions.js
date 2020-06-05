const DAPIAddress = require('./DAPIAddress');

const ListDAPIAddressProvider = require('./ListDAPIAddressProvider');

const SimplifiedMasternodeListProvider = require('../SimplifiedMasternodeListProvider/SimplifiedMasternodeListProvider');
const SimplifiedMasternodeListDAPIAddressProvider = require('./SimplifiedMasternodeListDAPIAddressProvider');

const JsonRpcTransport = require('../transport/JsonRpcTransport/JsonRpcTransport');
const requestJsonRpc = require('../transport/JsonRpcTransport/requestJsonRpc');

const DAPIClientError = require('../errors/DAPIClientError');

const networks = require('../networkConfigs');

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
  if (options.dapiAddressProvider) {
    if (options.addresses) {
      throw new DAPIClientError("Can't use 'address' with 'dapiAddressProvider' option");
    }

    if (options.seeds) {
      throw new DAPIClientError("Can't use 'seeds' with 'dapiAddressProvider' option");
    }

    if (options.network) {
      throw new DAPIClientError("Can't use 'network' with 'dapiAddressProvider' option");
    }

    return options.dapiAddressProvider;
  }

  if (options.addresses) {
    if (options.seeds) {
      throw new DAPIClientError("Can't use 'seeds' with 'addresses' option");
    }

    if (options.network) {
      throw new DAPIClientError("Can't use 'network' with 'addresses' option");
    }

    return new ListDAPIAddressProvider(
      options.addresses.map((rawAddress) => new DAPIAddress(rawAddress)),
      options,
    );
  }

  if (options.seeds) {
    if (options.network) {
      throw new DAPIClientError("Can't use 'network' with 'seeds' option");
    }

    const listDAPIAddressProvider = new ListDAPIAddressProvider(
      options.seeds.map((rawAddress) => new DAPIAddress(rawAddress)),
      options,
    );

    const jsonRpcTransport = new JsonRpcTransport(
      createDAPIAddressProviderFromOptions,
      requestJsonRpc,
      listDAPIAddressProvider,
      options,
    );

    const smlProvider = new SimplifiedMasternodeListProvider(
      jsonRpcTransport,
      { networkType: options.networkType },
    );

    return new SimplifiedMasternodeListDAPIAddressProvider(smlProvider, listDAPIAddressProvider);
  }

  if (options.network) {
    if (!networks[options.network]) {
      throw new DAPIClientError(`Invalid network '${options.network}'`);
    }

    const networkConfig = { ...options, ...networks[options.network] };
    // noinspection JSUnresolvedVariable
    delete networkConfig.network;

    return createDAPIAddressProviderFromOptions(networkConfig);
  }

  return null;
}

module.exports = createDAPIAddressProviderFromOptions;
