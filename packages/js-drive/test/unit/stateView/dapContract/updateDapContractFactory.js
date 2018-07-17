const updateDapContractFactory = require('../../../../lib/stateView/dapContract/updateDapContractFactory');

describe('updateDapContractFactory', () => {
  let dapContractRepository;
  let updateDapContract;

  beforeEach(function beforeEach() {
    dapContractRepository = {
      store: this.sinon.stub(),
    };
    updateDapContract = updateDapContractFactory(dapContractRepository);
  });

  it('should store DapContract', async () => {
    const dapId = '1234';
    const reference = {};
    const dapContract = {};
    updateDapContract(dapId, reference, dapContract);
    expect(dapContractRepository.store).to.calledOnce();
  });
});
