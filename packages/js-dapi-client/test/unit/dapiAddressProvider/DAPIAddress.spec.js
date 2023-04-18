const DAPIAddress = require('../../../lib/dapiAddressProvider/DAPIAddress');
const DAPIAddressHostMissingError = require(
  '../../../lib/dapiAddressProvider/errors/DAPIAddressHostMissingError',
);

describe('DAPIAddress', () => {
  let host;
  let port;

  beforeEach(() => {
    host = '127.0.0.1';
    port = DAPIAddress.DEFAULT_PORT + 1;
  });

  describe('#constructor', () => {
    it('should construct DAPIAddress from string with host and both ports', () => {
      const dapiAddress = new DAPIAddress(`${host}:${port}:no-ssl`);

      expect(dapiAddress).to.be.an.instanceOf(DAPIAddress);
      expect(dapiAddress.host).to.equal(host);
      expect(dapiAddress.port).to.equal(port);
      expect(dapiAddress.proRegTxHash).to.be.undefined();
      expect(dapiAddress.banCount).to.equal(0);
      expect(dapiAddress.banStartTime).to.be.undefined();
      expect(dapiAddress.protocol).to.equal('http');
    });

    it('should construct DAPIAddress from string with host and HTTP port', () => {
      const dapiAddress = new DAPIAddress(`${host}:${port}`);

      expect(dapiAddress).to.be.an.instanceOf(DAPIAddress);
      expect(dapiAddress.host).to.equal(host);
      expect(dapiAddress.port).to.equal(port);
      expect(dapiAddress.proRegTxHash).to.be.undefined();
      expect(dapiAddress.banCount).to.equal(0);
      expect(dapiAddress.banStartTime).to.be.undefined();
    });

    it('should construct DAPIAddress from DAPIAddress', () => {
      const address = new DAPIAddress(host);

      const dapiAddress = new DAPIAddress(address);

      expect(dapiAddress).to.be.an.instanceOf(DAPIAddress);
      expect(dapiAddress.toJSON()).to.deep.equal(address.toJSON());
    });

    it('should construct DAPIAddress form RawDAPIAddress', () => {
      const dapiAddress = new DAPIAddress({
        host,
      });

      expect(dapiAddress).to.be.an.instanceOf(DAPIAddress);
      expect(dapiAddress.host).to.equal(host);
      expect(dapiAddress.port).to.equal(DAPIAddress.DEFAULT_PORT);
      expect(dapiAddress.banCount).to.equal(0);
      expect(dapiAddress.banStartTime).to.be.undefined();
    });

    it('should construct DAPIAddress with defined ports', () => {
      const proRegTxHash = 'proRegTxHash';

      const dapiAddress = new DAPIAddress({
        host,
        port,
        proRegTxHash,
      });

      expect(dapiAddress).to.be.an.instanceOf(DAPIAddress);
      expect(dapiAddress.banCount).to.equal(0);
      expect(dapiAddress.banStartTime).to.be.undefined();
      expect(dapiAddress.toJSON()).to.deep.equal({
        host,
        port,
        proRegTxHash,
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });
    });

    it('should not set banCount and banStartTime from RawDAPIAddress', async () => {
      const dapiAddress = new DAPIAddress({
        host,
        banCount: 100,
        banStartTime: 1000,
      });

      expect(dapiAddress).to.be.an.instanceOf(DAPIAddress);
      expect(dapiAddress.banCount).to.equal(0);
      expect(dapiAddress.banStartTime).to.be.undefined();
    });

    it('should throw DAPIAddressHostMissingError if host is missed', () => {
      try {
        // eslint-disable-next-line no-new
        new DAPIAddress('');

        expect.fail('should throw DAPIAddressHostMissingError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DAPIAddressHostMissingError);
      }
    });
  });

  describe('#getHost', () => {
    it('should return host', () => {
      const dapiAddress = new DAPIAddress(host);

      expect(dapiAddress.getHost()).to.equal(host);
    });
  });

  describe('#setHost', () => {
    it('should set host', () => {
      const otherHost = '192.168.1.1';

      const dapiAddress = new DAPIAddress(host);
      dapiAddress.setHost(otherHost);

      expect(dapiAddress.host).to.equal(otherHost);
    });
  });

  describe('#getPort', () => {
    it('should get port', () => {
      const dapiAddress = new DAPIAddress({
        host,
        port,
      });

      expect(dapiAddress.getPort()).to.equal(port);
    });
  });

  describe('#setPort', () => {
    it('should set port', () => {
      const dapiAddress = new DAPIAddress(host);
      dapiAddress.setPort(port);

      expect(dapiAddress.getPort()).to.equal(port);
    });
  });

  describe('#getProRegTxHash', () => {
    it('should get ProRegTxHash', () => {
      const proRegTxHash = 'proRegTxHash';

      const dapiAddress = new DAPIAddress({
        host,
        proRegTxHash,
      });

      expect(dapiAddress.getProRegTxHash()).to.equal(proRegTxHash);
    });
  });

  describe('#getBanStartTime', () => {
    it('should get ban start time', () => {
      const now = Date.now();

      const dapiAddress = new DAPIAddress(host);
      dapiAddress.banStartTime = now;

      const banStartTime = dapiAddress.getBanStartTime();
      expect(banStartTime).to.equal(now);
    });
  });

  describe('#getProtocol', () => {
    it('should get protocol', () => {
      const dapiAddress = new DAPIAddress(host);

      dapiAddress.protocol = 'http';

      const protocol = dapiAddress.getProtocol();
      expect(protocol).to.equal('http');
    });
  });

  describe('#getBanCount', () => {
    it('should get ban count', () => {
      const dapiAddress = new DAPIAddress(host);
      dapiAddress.banCount = 666;

      const banCount = dapiAddress.getBanCount();
      expect(banCount).to.equal(666);
    });
  });

  describe('#markAsBanned', () => {
    it('should mark address as banned', () => {
      const dapiAddress = new DAPIAddress(host);
      dapiAddress.markAsBanned();

      expect(dapiAddress.banCount).to.equal(1);
      expect(dapiAddress.banStartTime).to.be.greaterThan(0);
    });
  });

  describe('#markAsLive', () => {
    it('should mark address as live', () => {
      const dapiAddress = new DAPIAddress(host);
      dapiAddress.banCount = 1;
      dapiAddress.banStartTime = Date.now();

      dapiAddress.markAsLive();

      expect(dapiAddress.banCount).to.equal(0);
      expect(dapiAddress.banStartTime).to.be.undefined();
    });
  });

  describe('#isBanned', () => {
    it('should return true if address is banned', () => {
      const dapiAddress = new DAPIAddress(host);

      dapiAddress.banCount = 1;

      const isBanned = dapiAddress.isBanned();
      expect(isBanned).to.be.true();
    });

    it('should return false if address is not banned', () => {
      const dapiAddress = new DAPIAddress(host);

      const isBanned = dapiAddress.isBanned();
      expect(isBanned).to.be.false();
    });
  });

  describe('#isSelfSigned', () => {
    it('should return true if address uses self signed certificate', () => {
      const dapiAddress = new DAPIAddress(host);

      dapiAddress.allowSelfSignedCertificate = true;

      const isSelfSigned = dapiAddress.isSelfSignedCertificateAllowed();
      expect(isSelfSigned).to.be.true();
    });

    it('should return true if address doesn\'t use self signed certificate', () => {
      const dapiAddress = new DAPIAddress(host);

      dapiAddress.allowSelfSignedCertificate = false;

      const isSelfSigned = dapiAddress.isSelfSignedCertificateAllowed();
      expect(isSelfSigned).to.be.false();
    });
  });

  describe('#toJSON', () => {
    it('should return RawDAPIAddress', () => {
      const dapiAddress = new DAPIAddress(host);

      expect(dapiAddress.toJSON()).to.deep.equal({
        host: dapiAddress.getHost(),
        port: dapiAddress.getPort(),
        proRegTxHash: dapiAddress.getProRegTxHash(),
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });
    });
  });

  describe('toString', () => {
    it('should return a string representation', () => {
      const dapiAddress = new DAPIAddress(host);

      const dapiAddressString = `${dapiAddress.getProtocol()}://${dapiAddress.getHost()}:`
        + `${dapiAddress.getPort()}`;

      expect(`${dapiAddress}`).to.equal(dapiAddressString);
    });
  });
});
