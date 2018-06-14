const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const storeDapContractFactory = require('../../../../lib/stateView/dapContract/storeDapContractFactory');

describe('storeDapContractFactory', () => {
  let dapContractRepository;
  let ipfsClient;
  beforeEach(function beforeEach() {
    dapContractRepository = {
      store: this.sinon.stub(),
    };
    ipfsClient = {
      dag: {
        get: this.sinon.stub(),
      },
    };
  });

  it('should throw an error if IPFS fails', async () => {
    ipfsClient.dag.get.throws(new Error('IpfsError'));
    const storeDapContract = storeDapContractFactory(dapContractRepository, ipfsClient);

    try {
      const cid = 'zdpuB1nHv2ewWb3k5dgm2FNuGsKohuujAg3uWJTopZsrxJiXG';
      await storeDapContract(cid);
    } catch (error) {
      expect(error.message).to.equal('IpfsError');
    }
  });

  xit('should throw an error if StateTransitionPacket fails', async () => {
    ipfsClient.dag.get.returns({ value: { packet: 'shit' } });
    const storeDapContract = storeDapContractFactory(dapContractRepository, ipfsClient);

    try {
      const cid = 'zdpuB1nHv2ewWb3k5dgm2FNuGsKohuujAg3uWJTopZsrxJiXG';
      await storeDapContract(cid);
    } catch (error) {
      expect(error.message).to.equal('InvalidPacket');
    }
  });

  it('should throw an error if DapContractRepository fails', async () => {
    dapContractRepository.store.throws(new Error('RepositoryFails'));
    const packet = getTransitionPacketFixtures()[0];
    ipfsClient.dag.get.returns({ value: packet });
    const storeDapContract = storeDapContractFactory(dapContractRepository, ipfsClient);

    try {
      const cid = 'zdpuB1nHv2ewWb3k5dgm2FNuGsKohuujAg3uWJTopZsrxJiXG';
      await storeDapContract(cid);
    } catch (error) {
      expect(error.message).to.equal('RepositoryFails');
    }
  });

  it('should return to work successfully', async () => {
    const packet = getTransitionPacketFixtures()[0];
    ipfsClient.dag.get.returns({ value: packet });
    const storeDapContract = storeDapContractFactory(dapContractRepository, ipfsClient);

    const cid = 'zdpuB1nHv2ewWb3k5dgm2FNuGsKohuujAg3uWJTopZsrxJiXG';
    await storeDapContract(cid);

    expect(dapContractRepository.store).to.be.calledOnce();
  });
});
