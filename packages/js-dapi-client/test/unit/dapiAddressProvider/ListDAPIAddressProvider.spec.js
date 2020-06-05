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
