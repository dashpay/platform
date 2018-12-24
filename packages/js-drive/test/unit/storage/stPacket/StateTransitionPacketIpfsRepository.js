const StateTransitionPacketIpfsRepository = require('../../../../lib/storage/stPacket/StateTransitionPacketIpfsRepository');

describe('StateTransitionPacketIpfsRepository', () => {
  let ipfsApiMock;
  let stPacketRepository;
  beforeEach(function beforeEach() {
    ipfsApiMock = {
      dag: {
        put: this.sinon.stub(),
        get: this.sinon.stub(),
      },
      pin: {
        add: this.sinon.stub(),
        rm: this.sinon.stub(),
        ls: this.sinon.stub(),
      },
    };

    stPacketRepository = new StateTransitionPacketIpfsRepository(ipfsApiMock, 1000);
  });

  it('should call IPFS dag.get method upon calling find', async () => {
    const somePacketData = {
      pver: 42,
    };
    const getPromise = Promise.resolve({
      value: somePacketData,
    });
    ipfsApiMock.dag.get.returns(getPromise);
    const cid = 'some_cid'; // not technically a CID

    const result = await stPacketRepository.find(cid);

    expect(ipfsApiMock.dag.get).to.be.calledOnce();
    expect(ipfsApiMock.dag.get).to.be.calledWith(cid);
    expect(result.toJSON({ skipMeta: true })).to.be.deep.equal(somePacketData);
  });

  it('should call IPFS dag.put method upon calling store', async () => {
    const somePacketData = {
      pver: 42,
    };
    const putPromise = Promise.resolve();
    ipfsApiMock.dag.put.returns(putPromise);
    const cid = 'some_cid'; // not technically a CID

    const packetMock = {
      getCID: () => cid,
      toJSON: () => somePacketData,
    };

    await stPacketRepository.store(packetMock);

    expect(ipfsApiMock.dag.put).to.be.calledOnce();
    expect(ipfsApiMock.dag.put).to.be.calledWith(somePacketData, { cid });
  });

  it('should call IPFS pin.add method upon calling download', async () => {
    const pinPromise = Promise.resolve();
    ipfsApiMock.pin.add.returns(pinPromise);
    const cid = {
      toBaseEncodedString: () => 'some_cid',
    }; // not technically a CID

    await stPacketRepository.download(cid);

    expect(ipfsApiMock.pin.add).to.be.calledOnce();
    expect(ipfsApiMock.pin.add).to.be.calledWith(cid.toBaseEncodedString(), { recursive: true });
  });

  it('should call IPFS pin.rm method upon calling delete', async () => {
    const pinPromise = Promise.resolve();
    ipfsApiMock.pin.rm.returns(pinPromise);
    const cid = {
      toBaseEncodedString: () => 'some_cid',
    }; // not technically a CID

    await stPacketRepository.delete(cid);

    expect(ipfsApiMock.pin.rm).to.be.calledOnce();
    expect(ipfsApiMock.pin.rm).to.be.calledWith(cid.toBaseEncodedString(), { recursive: true });
  });

  it('should call IPFS pin.ls and pin.rm methods upon calling deleteAll', async () => {
    const recursivePin = { type: 'recursive', hash: 'cid' };
    const pinLSPromise = Promise.resolve([
      recursivePin,
      { type: 'other', hash: 'other_cid' },
    ]);
    ipfsApiMock.pin.ls.returns(pinLSPromise);

    const pinRMPromise = Promise.resolve();
    ipfsApiMock.pin.rm.returns(pinRMPromise);

    await stPacketRepository.deleteAll();

    expect(ipfsApiMock.pin.ls).to.be.calledOnce();
    expect(ipfsApiMock.pin.rm).to.be.calledOnce();
    expect(ipfsApiMock.pin.rm).to.be.calledWith(recursivePin.hash);
  });
});
