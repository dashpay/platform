const createDAPIAddressProviderFromOptions = require(
  '../../../lib/dapiAddressProvider/createDAPIAddressProviderFromOptions',
);
const ListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/ListDAPIAddressProvider');
const SimplifiedMasternodeListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/SimplifiedMasternodeListDAPIAddressProvider');

const networks = require('../../../lib/networkConfigs');

const DAPIClientError = require('../../../lib/errors/DAPIClientError');

describe('createDAPIAddressProviderFromOptions', () => {
  describe('dapiAddressProvider', () => {
    let options;
    let dapiAddressProvider;

    beforeEach(() => {
      dapiAddressProvider = Object.create(null);

      options = {
        dapiAddressProvider,
      };
    });

    it('should return AddressProvider from `dapiAddressProvider` option', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.equal(dapiAddressProvider);
    });

    it('should throw DAPIClientError if `addresses` option is passed too', async () => {
      options.addresses = ['localhost'];

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

    it('should throw DAPIClientError if `network` option is passed too', async () => {
      options.network = 'testnet';

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });
  });

  describe('addresses', () => {
    let options;

    beforeEach(() => {
      options = {
        addresses: ['localhost'],
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

    it('should throw DAPIClientError if `network` option is passed too', async () => {
      options.network = 'testnet';

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
      };
    });

    it('should return SimplifiedMasternodeListDAPIAddressProvider based on seeds', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.be.an.instanceOf(SimplifiedMasternodeListDAPIAddressProvider);
    });

    it('should throw DAPIClientError if `network` option is passed too', async () => {
      options.network = 'testnet';

      try {
        createDAPIAddressProviderFromOptions(options);

        expect.fail('should throw DAPIClientError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIClientError);
      }
    });
  });

  describe('network', () => {
    let options;

    beforeEach(() => {
      options = {
        network: Object.keys(networks)[0],
      };
    });

    it('should create address provider from `network` options', async () => {
      const result = createDAPIAddressProviderFromOptions(options);

      expect(result).to.be.an.instanceOf(SimplifiedMasternodeListDAPIAddressProvider);
    });

    it('should throw DAPIClientError if `network` is invalid', async () => {
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
