const ListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/ListDAPIAddressProvider');
const DAPIAddress = require('../../../lib/dapiAddressProvider/DAPIAddress');

describe('ListDAPIAddressProvider', () => {
  let listDAPIAddressProvider;
  let addresses;
  let options;
  let bannedAddress;
  let notBannedAddress;

  beforeEach(() => {
    bannedAddress = new DAPIAddress('192.168.1.1');
    bannedAddress.markAsBanned();

    notBannedAddress = new DAPIAddress('192.168.1.2');

    addresses = [
      bannedAddress,
      notBannedAddress,
    ];

    options = {};

    listDAPIAddressProvider = new ListDAPIAddressProvider(
      addresses,
      options,
    );
  });

  describe('#constructor', () => {
    it('should set base ban time option', () => {
      const baseBanTime = 1000;

      listDAPIAddressProvider = new ListDAPIAddressProvider(
        addresses,
        { baseBanTime },
      );

      expect(listDAPIAddressProvider.options.baseBanTime).to.equal(baseBanTime);
    });

    it('should set default base ban time option if not passed', () => {
      listDAPIAddressProvider = new ListDAPIAddressProvider(
        addresses,
      );

      expect(listDAPIAddressProvider.options.baseBanTime).to.equal(60 * 1000);
    });
  });

  describe('#getLiveAddresses', () => {
    it('should return live addresses', () => {
      const bannedInThePastAddress = new DAPIAddress('192.168.1.3');
      bannedInThePastAddress.banCount = 1;
      bannedInThePastAddress.banStartTime = Date.now() - 3 * 60 * 1000;

      const bannedManyTimesAddress = new DAPIAddress('192.168.1.4');
      bannedManyTimesAddress.banCount = 3;
      bannedManyTimesAddress.banStartTime = Date.now() - 2 * 60 * 1000;

      listDAPIAddressProvider = new ListDAPIAddressProvider([
        bannedAddress,
        notBannedAddress,
        bannedInThePastAddress,
        bannedManyTimesAddress,
      ]);

      const liveAddresses = listDAPIAddressProvider.getLiveAddresses();

      expect(liveAddresses).to.have.lengthOf(2);
      expect(liveAddresses[0]).to.equal(notBannedAddress);
      expect(liveAddresses[1]).to.equal(bannedInThePastAddress);
    });

    it('should return empty array if all addresses are banned', () => {
      listDAPIAddressProvider.addresses.forEach((address) => {
        address.markAsBanned();
      });

      const liveAddresses = listDAPIAddressProvider.getLiveAddresses();

      expect(liveAddresses).to.have.lengthOf(0);
    });
  });

  describe('#getLiveAddress', () => {
    it('should return random live address', async () => {
      const address = await listDAPIAddressProvider.getLiveAddress();

      expect(address).to.equal(notBannedAddress);
    });

    it('should return undefined when there are no live addresses', async () => {
      listDAPIAddressProvider.addresses.forEach((address) => {
        address.markAsBanned();
      });

      const address = await listDAPIAddressProvider.getLiveAddress();

      expect(address).to.be.undefined();
    });

    it('should return modified address for a masternode-list node on localhost network', async () => {
      options = {
        network: 'local',
      };

      // Addresses discovered from the masternode list carry the masternode's
      // proRegTxHash and may hold a docker-internal IP that cannot be reached
      // from the host (macOS), so they are rewritten to the local gateway.
      const discoveredAddress = new DAPIAddress({
        host: '172.16.0.2',
        proRegTxHash: 'a'.repeat(64),
      });

      listDAPIAddressProvider = new ListDAPIAddressProvider(
        [discoveredAddress],
        options,
      );

      const liveAddress = await listDAPIAddressProvider.getLiveAddress();

      expect(liveAddress.host).to.equal('127.0.0.1');
      expect(liveAddress.protocol).to.equal('https');
      expect(liveAddress.allowSelfSignedCertificate).to.be.true();
    });

    it('should not modify a caller-supplied non-loopback address', async () => {
      options = {
        network: 'local',
      };

      // A caller-supplied address (no proRegTxHash — it did not come from the
      // masternode list) names the exact gateway to talk to, even when the
      // host is a secondary loopback, LAN IP, or container hostname.
      const explicitAddress = new DAPIAddress('127.0.0.2:45003:self-signed');

      listDAPIAddressProvider = new ListDAPIAddressProvider(
        [explicitAddress],
        options,
      );

      const liveAddress = await listDAPIAddressProvider.getLiveAddress();

      expect(liveAddress.host).to.equal('127.0.0.2');
      expect(liveAddress.port).to.equal(45003);
      expect(liveAddress.allowSelfSignedCertificate).to.be.true();
    });

    it('should not modify an explicitly configured loopback address', async () => {
      options = {
        network: 'local',
      };

      // A local network that moved its ports off the stock 2443 range
      // (dashmate e2e suites do) is addressed explicitly; rewriting the port
      // would redirect every request to whatever squats the stock ports.
      const loopbackAddress = new DAPIAddress('127.0.0.1:45003:self-signed');

      listDAPIAddressProvider = new ListDAPIAddressProvider(
        [loopbackAddress],
        options,
      );

      const liveAddress = await listDAPIAddressProvider.getLiveAddress();

      expect(liveAddress.host).to.equal('127.0.0.1');
      expect(liveAddress.port).to.equal(45003);
      expect(liveAddress.allowSelfSignedCertificate).to.be.true();
    });
  });

  describe('#hasLiveAddresses', () => {
    it('should return true if we have at least one unbanned address', async () => {
      const hasAddresses = await listDAPIAddressProvider.hasLiveAddresses();

      expect(hasAddresses).to.be.true();
    });

    it('should return false if all addresses are banned', async () => {
      listDAPIAddressProvider.addresses.forEach((address) => {
        address.markAsBanned();
      });

      const hasAddresses = await listDAPIAddressProvider.hasLiveAddresses();

      expect(hasAddresses).to.be.false();
    });
  });

  describe('#getAllAddresses', () => {
    it('should get all addresses', () => {
      const allAddresses = listDAPIAddressProvider.getAllAddresses();

      expect(allAddresses).to.deep.equal(listDAPIAddressProvider.addresses);
    });
  });

  describe('#setAddresses', () => {
    it('should set addresses and overwrite previous', () => {
      addresses = [
        notBannedAddress,
      ];
      listDAPIAddressProvider.setAddresses(addresses);

      expect(listDAPIAddressProvider.addresses).to.deep.equal(addresses);
    });
  });
});
