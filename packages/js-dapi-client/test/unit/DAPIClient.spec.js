const DAPIClient = require('../../lib/DAPIClient');
const CoreMethodsFacade = require('../../lib/methods/core/CoreMethodsFacade');
const PlatformMethodsFacade = require('../../lib/methods/platform/PlatformMethodsFacade');
const SimplifiedMasternodeListDAPIAddressProvider = require('../../lib/dapiAddressProvider/SimplifiedMasternodeListDAPIAddressProvider');
const ListDAPIAddressProvider = require('../../lib/dapiAddressProvider/ListDAPIAddressProvider');
const BlockHeadersProvider = require('../../lib/BlockHeadersProvider/BlockHeadersProvider');

describe('DAPIClient', () => {
  let options;
  let dapiClient;

  describe('#constructor', () => {
    it('should construct DAPIClient with options', async () => {
      options = {
        retries: 0,
        newOption: true,
      };

      dapiClient = new DAPIClient(options);

      expect(dapiClient.options).to.deep.equal({
        network: 'testnet',
        retries: 0,
        newOption: true,
        timeout: 10000,
        blockHeadersProviderOptions: BlockHeadersProvider.defaultOptions,
      });

      expect(dapiClient.dapiAddressProvider).to.be.an.instanceOf(
        SimplifiedMasternodeListDAPIAddressProvider,
      );

      expect(dapiClient.blockHeadersProvider).to.be.an.instanceOf(
        BlockHeadersProvider,
      );

      expect(dapiClient.core).to.be.an.instanceOf(CoreMethodsFacade);
      expect(dapiClient.platform).to.be.an.instanceOf(PlatformMethodsFacade);
    });

    it('should construct DAPIClient without options', async () => {
      dapiClient = new DAPIClient();

      expect(dapiClient.options).to.deep.equal({
        retries: 5,
        timeout: 10000,
        network: 'testnet',
        blockHeadersProviderOptions: BlockHeadersProvider.defaultOptions,
      });

      expect(dapiClient.dapiAddressProvider).to.be.an.instanceOf(
        SimplifiedMasternodeListDAPIAddressProvider,
      );

      expect(dapiClient.blockHeadersProvider).to.be.an.instanceOf(
        BlockHeadersProvider,
      );

      expect(dapiClient.core).to.be.an.instanceOf(CoreMethodsFacade);
      expect(dapiClient.platform).to.be.an.instanceOf(PlatformMethodsFacade);
    });

    it('should construct DAPIClient with address options', async () => {
      options = {
        retries: 0,
        dapiAddresses: ['localhost'],
      };

      dapiClient = new DAPIClient(options);

      expect(dapiClient.options).to.deep.equal({
        retries: 0,
        dapiAddresses: ['localhost'],
        network: 'testnet',
        timeout: 10000,
        blockHeadersProviderOptions: BlockHeadersProvider.defaultOptions,
      });

      expect(dapiClient.dapiAddressProvider).to.be.an.instanceOf(ListDAPIAddressProvider);

      expect(dapiClient.blockHeadersProvider).to.be.an.instanceOf(
        BlockHeadersProvider,
      );

      expect(dapiClient.core).to.be.an.instanceOf(CoreMethodsFacade);
      expect(dapiClient.platform).to.be.an.instanceOf(PlatformMethodsFacade);
    });

    it('should construct DAPIClient with network options', async () => {
      options = {
        retries: 3,
        network: 'local',
      };

      dapiClient = new DAPIClient(options);
      expect(dapiClient.options).to.deep.equal({
        retries: 3,
        network: 'local',
        timeout: 10000,
        blockHeadersProviderOptions: BlockHeadersProvider.defaultOptions,
      });

      expect(dapiClient.dapiAddressProvider).to.be.an.instanceOf(ListDAPIAddressProvider);
      expect(dapiClient.blockHeadersProvider).to.be.an.instanceOf(
        BlockHeadersProvider,
      );
      expect(dapiClient.core).to.be.an.instanceOf(CoreMethodsFacade);
      expect(dapiClient.platform).to.be.an.instanceOf(PlatformMethodsFacade);
    });
  });
});
