const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');

const getMNListDiffsFixture = require('../../../lib/test/fixtures/getMNListDiffsFixture');
const DAPIAddress = require('../../../lib/dapiAddressProvider/DAPIAddress');

const SimplifiedMasternodeListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/SimplifiedMasternodeListDAPIAddressProvider');

describe('SimplifiedMasternodeListDAPIAddressProvider', () => {
  let smlDAPIAddressProvider;
  let smlProviderMock;
  let listDAPIAddressProviderMock;
  let smlMock;
  let validMasternodeList;
  let addresses;

  beforeEach(function beforeEach() {
    const [mnListDiffFixture] = getMNListDiffsFixture();

    validMasternodeList = [
      new SimplifiedMNListEntry(mnListDiffFixture.mnList[0]),
      new SimplifiedMNListEntry(mnListDiffFixture.mnList[1]),
      new SimplifiedMNListEntry(mnListDiffFixture.mnList[2]),
    ];

    addresses = [
      new DAPIAddress({
        host: validMasternodeList[0].getIp(),
        proRegTxHash: validMasternodeList[0].proRegTxHash,
      }),
      new DAPIAddress({
        host: '127.0.0.1',
        proRegTxHash: validMasternodeList[1].proRegTxHash,
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
    );
  });

  describe('#getLiveAddress', () => {
    it('should get SML from provider, update ListAddressProvider and return live address', async () => {
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
        httpPort: DAPIAddress.DEFAULT_HTTP_PORT,
        grpcPort: DAPIAddress.DEFAULT_GRPC_PORT,
        proRegTxHash: validMasternodeList[0].proRegTxHash,
      });

      expect(secondAddress).to.be.instanceOf(DAPIAddress);
      expect(secondAddress).to.equal(addresses[1]);
      expect(secondAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[1].getIp(),
        httpPort: DAPIAddress.DEFAULT_HTTP_PORT,
        grpcPort: DAPIAddress.DEFAULT_GRPC_PORT,
        proRegTxHash: validMasternodeList[1].proRegTxHash,
      });

      expect(thirdAddress).to.be.instanceOf(DAPIAddress);
      expect(thirdAddress).to.not.equal(addresses[2]);
      expect(thirdAddress.toJSON()).to.deep.equal({
        host: validMasternodeList[2].getIp(),
        httpPort: DAPIAddress.DEFAULT_HTTP_PORT,
        grpcPort: DAPIAddress.DEFAULT_GRPC_PORT,
        proRegTxHash: validMasternodeList[2].proRegTxHash,
      });

      expect(smlMock.getValidMasternodesList).to.be.calledOnceWithExactly();
      expect(smlProviderMock.getSimplifiedMNList).to.be.calledOnceWithExactly();
      expect(listDAPIAddressProviderMock.getAllAddresses).to.be.calledOnceWithExactly();
      expect(listDAPIAddressProviderMock.getLiveAddress).to.be.calledOnceWithExactly();
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
