const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');

const DAPIAddress = require('../../../lib/dapiAddressProvider/DAPIAddress');

const SimplifiedMasternodeListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/SimplifiedMasternodeListDAPIAddressProvider');
const ListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/ListDAPIAddressProvider');

describe('SimplifiedMasternodeListDAPIAddressProvider', () => {
  let smlDAPIAddressProvider;
  let smlProviderMock;
  let listDAPIAddressProviderMock;
  let smlMock;
  let validMasternodeList;
  let addresses;
  let mnListDiffFixture;

  beforeEach(function beforeEach() {
    mnListDiffFixture = [{
      proRegTxHash: 'f5ec54aed788c434da2fc535ea6b125ec6fc54e58bc0a00a005d1a8d5e477a90',
      confirmedHash: '53125505b0e9d11b371cf3e12c92d164296dfa215fde6201d28ea44bed992187',
      service: '192.168.65.2:20101',
      pubKeyOperator: '951a3208ba531ea75aedd2dc0a9efc75f2c4d9492f1ee0a989b593bcd9722b1a101774d80a426552a9f91d24eb55af6e',
      votingAddress: 'yYH1rgZsgvkmT8bSSSw1cKCjyVPnFpTBCw',
      isValid: true,
      nVersion: 2,
      nType: 1,
      payoutAddress: 'yZv7wf496sjqJVgnEUAtYKozWQhVpoHRh9',
      platformHTTPPort: 3200,
      platformNodeID: 'fe84df23e1a7f1db40e8e3fd3a4d88662bf0d89d',
    }, {
      proRegTxHash: 'a2c9b34ef525271d84f70a0d4d2c107e8a2f81cd4d8256dc7b3911ed253d5611',
      confirmedHash: '29ff8afb463604ba7d984b483e92dfefa4e80e12de3acae6d75f9b910df9eab6',
      service: '192.168.65.2:20201',
      pubKeyOperator: 'a5ad6d8cad7b233210b718a5fc9ec3cea18aeebe38b2e3122deb581e430aa28875fe7336c283871db42808f8d4107745',
      votingAddress: 'yRXtaRmQ7LCmT5XcgzQdLwPEf31dycBaeY',
      isValid: true,
      nVersion: 2,
      nType: 1,
      payoutAddress: 'yiBP17AgHGit2TE9p9FpHEh4ouowNSxMxg',
      platformHTTPPort: 3200,
      platformNodeID: 'fe84df23e1a7f1db40e8e3fd3a4d88662bf0d89d',
    }, {
      proRegTxHash: '1c81a5faa2c0e0d96eb59c58a10fcbc87f431bb6cd880d960b43b269e682d2d2',
      confirmedHash: '03cc2acc135ab51304d3cff42215c7a8041902fa3f19451d5562a03b38143e8f',
      service: '192.168.65.2:20001',
      pubKeyOperator: '96f83eedc8a7b87663e591987f051ce341a6fb88989322c64bbbf56d205e4e77d2cb7d839d8b4106a8a1f5d5cf7cfa57',
      votingAddress: 'ybJfuKs59MJWkPEnS8qNmtvdisHrCy7Njn',
      isValid: true,
      nVersion: 2,
      nType: 1,
      payoutAddress: 'yd3AnRA5YRtN1jsv7jqUK8egA6Mk9e8HoS',
      platformHTTPPort: 3200,
      platformNodeID: 'fe84df23e1a7f1db40e8e3fd3a4d88662bf0d89d',
    }];

    validMasternodeList = [
      new SimplifiedMNListEntry(mnListDiffFixture[0]),
      new SimplifiedMNListEntry(mnListDiffFixture[1]),
      new SimplifiedMNListEntry(mnListDiffFixture[2]),
    ];

    addresses = [
      new DAPIAddress({
        host: validMasternodeList[0].getIp(),
        proRegTxHash: validMasternodeList[0].proRegTxHash,
        port: validMasternodeList[0].platformHTTPPort,
      }),
      new DAPIAddress({
        host: '127.0.0.1',
        proRegTxHash: validMasternodeList[1].proRegTxHash,
        port: validMasternodeList[1].platformHTTPPort,
      }),
      new DAPIAddress({
        host: '127.0.0.1',
      }),
    ];

    smlMock = {
      getValidMasternodesList: this.sinon.stub().returns(validMasternodeList),
    };

    smlProviderMock = {
      getSimplifiedMNList: this.sinon.stub().resolves(smlMock),
    };

    listDAPIAddressProviderMock = {
      getLiveAddress: this.sinon.stub().resolves(addresses[0]),
      hasLiveAddresses: this.sinon.stub().resolves(true),
      getAllAddresses: this.sinon.stub().returns(addresses),
      setAddresses: this.sinon.stub(),
    };

    smlDAPIAddressProvider = new SimplifiedMasternodeListDAPIAddressProvider(
      smlProviderMock,
      listDAPIAddressProviderMock,
      [],
    );
  });

  describe('#getLiveAddress', () => {
    it('should return live address', async () => {
      const liveAddress = await smlDAPIAddressProvider.getLiveAddress();

      expect(liveAddress).to.equal(addresses[0]);

      expect(listDAPIAddressProviderMock.setAddresses).to.be.calledOnce();

      expect(listDAPIAddressProviderMock.setAddresses.getCall(0).args).to.have.lengthOf(1);
      expect(listDAPIAddressProviderMock.setAddresses.getCall(0).args[0]).to.be.an('array');
      expect(listDAPIAddressProviderMock.setAddresses.getCall(0).args[0]).to.have.lengthOf(3);

      const [
        firstAddress,
        secondAddress,
        thirdAddress,
      ] = listDAPIAddressProviderMock.setAddresses.getCall(0).args[0];

      expect(firstAddress).to.be.instanceOf(DAPIAddress);
      expect(firstAddress).to.equal(addresses[0]);
      expect(firstAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[0].getIp(),
        port: validMasternodeList[0].platformHTTPPort,
        proRegTxHash: validMasternodeList[0].proRegTxHash,
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });

      expect(secondAddress).to.be.instanceOf(DAPIAddress);
      expect(secondAddress).to.deep.equal(addresses[1]);
      expect(secondAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[1].getIp(),
        port: validMasternodeList[1].platformHTTPPort,
        proRegTxHash: validMasternodeList[1].proRegTxHash,
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });

      expect(thirdAddress).to.be.instanceOf(DAPIAddress);
      expect(thirdAddress).to.not.equal(addresses[2]);
      expect(thirdAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[2].getIp(),
        port: validMasternodeList[2].platformHTTPPort,
        proRegTxHash: validMasternodeList[2].proRegTxHash,
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });

      expect(smlMock.getValidMasternodesList).to.be.calledOnceWithExactly();
      expect(smlProviderMock.getSimplifiedMNList).to.be.calledOnceWithExactly();
      expect(listDAPIAddressProviderMock.getAllAddresses).to.be.calledOnceWithExactly();
      expect(listDAPIAddressProviderMock.getLiveAddress).to.be.calledOnceWithExactly();
    });

    it('should return filtered live address', async () => {
      smlDAPIAddressProvider = new SimplifiedMasternodeListDAPIAddressProvider(
        smlProviderMock,
        listDAPIAddressProviderMock,
        [new DAPIAddress(`${validMasternodeList[1].getIp()}:${validMasternodeList[1].platformHTTPPort}`)],
      );

      await smlDAPIAddressProvider.getLiveAddress();

      expect(listDAPIAddressProviderMock.setAddresses).to.be.calledOnce();

      expect(listDAPIAddressProviderMock.setAddresses.getCall(0).args).to.have.lengthOf(1);
      expect(listDAPIAddressProviderMock.setAddresses.getCall(0).args[0]).to.be.an('array');
      expect(listDAPIAddressProviderMock.setAddresses.getCall(0).args[0]).to.have.lengthOf(3);

      const [
        secondAddress,
        thirdAddress,
      ] = listDAPIAddressProviderMock.setAddresses.getCall(0).args[0];

      expect(secondAddress).to.be.instanceOf(DAPIAddress);
      expect(secondAddress).to.equal(addresses[0]);
      expect(secondAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[0].getIp(),
        port: validMasternodeList[0].platformHTTPPort,
        proRegTxHash: validMasternodeList[0].proRegTxHash,
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });

      expect(thirdAddress).to.be.instanceOf(DAPIAddress);
      expect(thirdAddress).to.not.equal(addresses[2]);
      expect(thirdAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[1].getIp(),
        port: validMasternodeList[1].platformHTTPPort,
        proRegTxHash: validMasternodeList[1].proRegTxHash,
        protocol: DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: false,
      });

      expect(smlMock.getValidMasternodesList).to.be.calledOnceWithExactly();
      expect(smlProviderMock.getSimplifiedMNList).to.be.calledOnceWithExactly();
      expect(listDAPIAddressProviderMock.getAllAddresses).to.be.calledOnceWithExactly();
      expect(listDAPIAddressProviderMock.getLiveAddress).to.be.calledOnceWithExactly();
    });
  });

  describe('#getLiveAddress on a regtest network', () => {
    const options = { network: 'local' };

    it('should rewrite a masternode-list address to the local gateway', async () => {
      // The fixture masternodes registered a docker-internal IP, which the
      // host cannot reach on macOS.
      smlDAPIAddressProvider = new SimplifiedMasternodeListDAPIAddressProvider(
        smlProviderMock,
        new ListDAPIAddressProvider([], options),
        [],
        options,
      );

      const liveAddress = await smlDAPIAddressProvider.getLiveAddress();

      expect(liveAddress.getProRegTxHash()).to.be.oneOf(
        validMasternodeList.map((smlEntry) => smlEntry.proRegTxHash),
      );
      expect(liveAddress.getHost()).to.equal('127.0.0.1');
      expect(liveAddress.getPort()).to.be.oneOf([2443, 2543, 2643]);
      expect(liveAddress.getProtocol()).to.equal('https');
      expect(liveAddress.isSelfSignedCertificateAllowed()).to.be.true();
    });

    it('should rewrite a loopback masternode-list address to the self-signed local gateway', async () => {
      // A loopback host still needs the gateway's port and self-signed TLS.
      smlMock.getValidMasternodesList.returns([
        new SimplifiedMNListEntry({ ...mnListDiffFixture[2], service: '127.0.0.1:20001' }),
      ]);
      smlDAPIAddressProvider = new SimplifiedMasternodeListDAPIAddressProvider(
        smlProviderMock,
        new ListDAPIAddressProvider([], options),
        [],
        options,
      );

      const liveAddress = await smlDAPIAddressProvider.getLiveAddress();

      expect(liveAddress.getHost()).to.equal('127.0.0.1');
      expect(liveAddress.getPort()).to.equal(2443);
      expect(liveAddress.isSelfSignedCertificateAllowed()).to.be.true();
    });

    it('should not rewrite a masternode-list address outside regtest', async () => {
      // Mainnet: dashcore-lib's regtest is a global flag on its testnet object,
      // which the regtest cases above have switched on.
      const mainnetOptions = { network: 'mainnet' };
      smlDAPIAddressProvider = new SimplifiedMasternodeListDAPIAddressProvider(
        smlProviderMock,
        new ListDAPIAddressProvider([], mainnetOptions),
        [],
        mainnetOptions,
      );

      const liveAddress = await smlDAPIAddressProvider.getLiveAddress();

      expect(liveAddress.getHost()).to.equal(validMasternodeList[0].getIp());
      expect(liveAddress.getPort()).to.equal(validMasternodeList[0].platformHTTPPort);
      expect(liveAddress.isSelfSignedCertificateAllowed()).to.be.false();
    });
  });

  describe('#hasLiveAddresses', () => {
    it('should return ListAddressProvider#hasLiveAddresses result', async () => {
      const result = await smlDAPIAddressProvider.hasLiveAddresses();

      expect(result).to.be.true();

      expect(listDAPIAddressProviderMock.hasLiveAddresses).to.be.calledOnceWithExactly();
    });
  });
});
