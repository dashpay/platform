const createDAPIAddressProviderFromOptions = require(
  '../../../lib/dapiAddressProvider/createDAPIAddressProviderFromOptions',
);
const DAPIAddress = require('../../../lib/dapiAddressProvider/DAPIAddress');
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

    it('should not rewrite a caller-supplied non-default regtest address', async () => {
      options.dapiAddresses = ['127.0.0.2:45003:self-signed'];

      const provider = createDAPIAddressProviderFromOptions(options);

      const liveAddress = await provider.getLiveAddress();

      expect(liveAddress.getHost()).to.equal('127.0.0.2');
      expect(liveAddress.getPort()).to.equal(45003);
    });

    // An explicit list is never rewritten, even when its entries name the
    // masternode they belong to.
    it('should not rewrite an explicit raw address that carries a proRegTxHash', async () => {
      options.dapiAddresses = [{
        host: '127.0.0.2',
        port: 45003,
        allowSelfSignedCertificate: true,
        proRegTxHash: 'c'.repeat(64),
      }];

      const liveAddress = await createDAPIAddressProviderFromOptions(options).getLiveAddress();

      expect(liveAddress.getHost()).to.equal('127.0.0.2');
      expect(liveAddress.getPort()).to.equal(45003);
      expect(liveAddress.isSelfSignedCertificateAllowed()).to.be.true();
    });

    it('should not rewrite an explicit DAPIAddress that carries a proRegTxHash', async () => {
      options.dapiAddresses = [new DAPIAddress({
        host: '10.0.0.5',
        port: 45003,
        proRegTxHash: 'd'.repeat(64),
      })];

      const liveAddress = await createDAPIAddressProviderFromOptions(options).getLiveAddress();

      expect(liveAddress.getHost()).to.equal('10.0.0.5');
      expect(liveAddress.getPort()).to.equal(45003);
      expect(liveAddress.isSelfSignedCertificateAllowed()).to.be.false();
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

    it('should connect the `local` preset to the self-signed local gateway', async () => {
      const liveAddress = await createDAPIAddressProviderFromOptions({ network: 'local' })
        .getLiveAddress();

      expect(liveAddress.getHost()).to.equal('127.0.0.1');
      expect(liveAddress.getPort()).to.equal(2443);
      expect(liveAddress.getProtocol()).to.equal('https');
      expect(liveAddress.isSelfSignedCertificateAllowed()).to.be.true();
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
