const fetchDapContractMethodFactory = require('../../../../lib/api/methods/fetchDapContractMethodFactory');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');

describe('fetchDapContractMehotd', () => {
  let fetchDapContractMethod;
  let dapContractMongoDbRepository;

  beforeEach(function beforeEach() {
    dapContractMongoDbRepository = {
      find: this.sinon.stub(),
    };
    fetchDapContractMethod = fetchDapContractMethodFactory(dapContractMongoDbRepository);
  });

  it('should throw InvalidParamsError if DAP id is not provided', () => {
    expect(fetchDapContractMethod()).to.be.rejectedWith(InvalidParamsError);
  });

  it('should return DAP contract', async () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const dapName = 'DashPay';
    const packetHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const schema = {};
    const contract = new DapContract(dapId, dapName, packetHash, schema);
    dapContractMongoDbRepository.find.returns(contract);

    const dapContract = await fetchDapContractMethod({ dapId });

    expect(dapContract).to.be.deep.equal({
      dapId,
      dapName,
      packetHash,
      schema,
    });
  });
});
