const fetchDapContractMethodFactory = require('../../../../lib/api/methods/fetchDapContractMethodFactory');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');
const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');

describe('fetchDapContractMethod', () => {
  let fetchDapContractMethod;
  let dapContractMongoDbRepository;

  beforeEach(function beforeEach() {
    dapContractMongoDbRepository = {
      find: this.sinon.stub(),
    };
    fetchDapContractMethod = fetchDapContractMethodFactory(dapContractMongoDbRepository);
  });

  it('should throw InvalidParamsError if DAP id is not provided', () => {
    expect(fetchDapContractMethod({})).to.be.rejectedWith(InvalidParamsError);
  });

  it('should return DAP contract', async () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const dapName = 'DashPay';
    const reference = new Reference();
    const schema = {};
    const version = 2;
    const isDeleted = false;
    const previousVersions = [];
    const contract = new DapContract(
      dapId,
      dapName,
      reference,
      schema,
      version,
      isDeleted,
      previousVersions,
    );
    dapContractMongoDbRepository.find.returns(contract);

    const dapContract = await fetchDapContractMethod({ dapId });

    expect(dapContract).to.be.deep.equal({
      dapId,
      dapName,
      reference,
      schema,
      version,
      isDeleted,
      previousVersions,
    });
  });
  it('should throw error if DAP Contract not found', async () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';

    dapContractMongoDbRepository.find.returns(null);

    expect(fetchDapContractMethod({ dapId })).to.be.rejectedWith(InvalidParamsError);
  });
});
