const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const bs58 = require('bs58');

const DashPlatformProtocol = require('@dashevo/dpp');

const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');

const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');

describe('SVContractMongoDbRepository', () => {
  let svContractRepository;
  let svContract;
  let mongoDatabase;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    svContract = getSVContractFixture();

    svContractRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);
  });

  it('should store SV Contract entity', async () => {
    await svContractRepository.store(svContract);

    const result = await svContractRepository.find(svContract.getContractId());

    expect(result.toJSON()).to.deep.equal(svContract.toJSON());
  });

  it('should return null if SV Contract is not found', async () => {
    const result = await svContractRepository.find('unknown');

    expect(result).to.be.null();
  });

  it('should find all contracts by stHash', async () => {
    await svContractRepository.store(svContract);

    const svContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
    );

    expect(svContracts.length).to.equal(1);
    expect(svContracts).to.deep.equal([svContract]);
  });

  it('should return null if contract was marked as deleted', async () => {
    svContract.markAsDeleted();

    await svContractRepository.store(svContract);

    const contract = await svContractRepository.find(svContract.getContractId());

    expect(contract).to.be.null();
  });

  it('should use base58-encoded Contract ID as MongoDB document ID', async () => {
    await svContractRepository.store(svContract);

    const result = await mongoDatabase.collection('contracts').findOne({
      _id: bs58.encode(Buffer.from(svContract.getContractId(), 'hex')),
    });

    expect(result).to.be.not.null();
  });

  it('should successfuly delete a contract');
});
