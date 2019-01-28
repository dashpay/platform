const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const bs58 = require('bs58');

const DashPlatformProtocol = require('@dashevo/dpp');

const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');

const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');

describe('SVContractMongoDbRepository', () => {
  let svContractRepository;
  let svContract;
  let mongoDb;

  startMongoDb().then((mongoDbInstance) => {
    mongoDb = mongoDbInstance.getDb();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    svContract = getSVContractFixture();

    svContractRepository = new SVContractMongoDbRepository(mongoDb, dpp);
  });

  it('should store SV Contract entity', async () => {
    await svContractRepository.store(svContract);

    const result = await svContractRepository.find(svContract.getContractId());

    expect(result.toJSON()).to.deep.equal(svContract.toJSON());
  });

  it('should return null if not found', async () => {
    const result = await svContractRepository.find('unknown');

    expect(result).to.be.null();
  });

  it('should find all contracts by stHash', async () => {
    await svContractRepository.store(svContract);

    const svContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
    );

    expect(svContracts.length).to.be.equal(1);
    expect(svContracts).to.be.deep.equal([svContract]);
  });

  it('should return null if contract was marked as deleted', async () => {
    svContract.markAsDeleted();

    await svContractRepository.store(svContract);

    const contract = await svContractRepository.find(svContract.getContractId());

    expect(contract).to.be.null();
  });

  it('should use base58-encoded Contract ID as MongoDB document ID', async () => {
    await svContractRepository.store(svContract);

    const result = await mongoDb.collection('contracts').findOne({
      _id: bs58.encode(Buffer.from(svContract.getContractId(), 'hex')),
    });

    expect(result).not.to.be.null();
  });

  it('should successfuly delete a contract');
});
