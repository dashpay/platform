const createDAPIAddressProviderFromOptions = require(
  '../../../lib/dapiAddressProvider/createDAPIAddressProviderFromOptions',
);
const ListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/ListDAPIAddressProvider');
const SimplifiedMasternodeListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/SimplifiedMasternodeListDAPIAddressProvider');

const networkConfigs = require('../../../lib/networkConfigs');

const DAPIClientError = require('../../../lib/errors/DAPIClientError');

describe('createDAPIAddressProviderFromOptions', () => {
  describe('dapiAddressProvider', () => {
    let options;
    let dapiAddressProvider;

    beforeEach(() => {
      dapiAddressProvider = Object.create(null);

      options = {
        network: 'evonet',
        dapiAddressProvider,
      };
    });

    it('should return AddressProvider from `dapiAddressProvider` option', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.equal(dapiAddressProvider);
    });

    it('should throw DAPIClientError if `dapiAddresses` option is passed too', async () => {
      options.dapiAddresses = ['localhost'];

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });

    it('should throw DAPIClientError if `seeds` option is passed too', async () => {
      options.seeds = ['127.0.0.1'];

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });

    it('should throw DAPIClientError if `dapiAddressesWhiteList` option is passed too', async () => {
      options.dapiAddressesWhiteList = ['127.0.0.1'];

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });
  });

  describe('dapiAddresses', () => {
    let options;

    beforeEach(() => {
      options = {
        dapiAddresses: ['localhost'],
        network: 'local',
      };
    });

    it('should return ListDAPIAddressProvider with addresses', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.be.an.instanceOf(ListDAPIAddressProvider);
    });

    it('should throw DAPIClientError if `seeds` option is passed too', async () => {
      options.seeds = ['127.0.0.1'];

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });

    it('should throw DAPIClientError if `dapiAddressesWhiteList` option is passed too', async () => {
      options.dapiAddressesWhiteList = ['127.0.0.1'];

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });
  });

  describe('seeds', () => {
    let options;

    beforeEach(() => {
      options = {
        seeds: ['127.0.0.1'],
        network: 'local',
        loggerOptions: {
          identifier: '',
        },
      };
    });

    it('should return SimplifiedMasternodeListDAPIAddressProvider based on seeds', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.be.an.instanceOf(SimplifiedMasternodeListDAPIAddressProvider);
    });
  });

  describe('network', () => {
    let options;

    beforeEach(() => {
      options = {
        network: Object.keys(networkConfigs)[0],
        loggerOptions: {
          identifier: '',
        },
      };
    });

    it('should create address provider from `network` options', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.be.an.instanceOf(SimplifiedMasternodeListDAPIAddressProvider);
    });

    it('should throw DAPIClientError if there is no config for a specified network', async () => {
      options.network = 'unknown';

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });
  });
});
