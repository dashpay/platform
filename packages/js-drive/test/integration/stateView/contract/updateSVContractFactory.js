const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const Revision = require('../../../../lib/stateView/revisions/Revision');
const SVContract = require('../../../../lib/stateView/contract/SVContract');

const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');

const updateSVContractFactory = require('../../../../lib/stateView/contract/updateSVContractFactory');

const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('updateSVContractFactory', () => {
  let svContractRepository;
  let updateSVContract;
  let mongoDb;
  let svContract;
  let dpp;
  let contractId;
  let dpContract;

  startMongoDb().then((mongoDbInstance) => {
    mongoDb = mongoDbInstance.getDb();
  });

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    svContract = getSVContractFixture();
    dpContract = svContract.getDPContract();

    contractId = svContract.getContractId();

    svContractRepository = new SVContractMongoDbRepository(mongoDb, dpp);
    updateSVContract = updateSVContractFactory(svContractRepository);
  });

  it('should store SVContract', async () => {
    await updateSVContract(
      svContract.getContractId(),
      svContract.getReference(),
      svContract.getDPContract(),
    );

    const fetchedSVContract = await svContractRepository.find(svContract.getContractId());

    expect(fetchedSVContract).to.deep.equal(svContract);
  });

  it('should maintain SVContract previous revisions and add new one', async () => {
    // Create and store the second contract version
    const secondDPOContract = dpp.contract.createFromObject(dpContract.toJSON());
    secondDPOContract.setVersion(2);

    const secondSVContract = new SVContract(
      contractId,
      dpContract,
      getReferenceFixture(2),
      false,
      [svContract.getCurrentRevision()],
    );

    await svContractRepository.store(secondSVContract);

    // Update to the third contract version
    const thirdDPContract = dpp.contract.createFromObject(dpContract.toJSON());
    thirdDPContract.setVersion(3);

    await updateSVContract(
      contractId,
      getReferenceFixture(3),
      thirdDPContract,
    );

    const thirdSVContract = await svContractRepository.find(contractId);

    expect(thirdSVContract).to.be.instanceOf(SVContract);
    expect(thirdSVContract.getDPContract()).to.deep.equal(thirdDPContract);
    expect(thirdSVContract.getPreviousRevisions()).to.deep.equal([
      svContract.getCurrentRevision(),
      secondSVContract.getCurrentRevision(),
    ]);
  });

  it('should remove unnecessary previous versions of SVContract upon reverting', async () => {
    // Create and store the third contract version
    const thirdDPOContract = dpp.contract.createFromObject(dpContract.toJSON());
    thirdDPOContract.setVersion(3);

    const firstRevision = new Revision(1, getReferenceFixture(1));
    const secondRevision = new Revision(2, getReferenceFixture(2));

    const thirdSVContract = new SVContract(
      contractId,
      thirdDPOContract,
      getReferenceFixture(3),
      false,
      [firstRevision, secondRevision],
    );

    await svContractRepository.store(thirdSVContract);

    // Revert to second contract version
    const secondDPContract = dpp.contract.createFromObject(dpContract.toJSON());
    secondDPContract.setVersion(2);

    await updateSVContract(
      contractId,
      secondRevision.getReference(),
      secondDPContract,
      true,
    );

    const secondSVContract = await svContractRepository.find(contractId);

    expect(secondSVContract).to.be.instanceOf(SVContract);
    expect(secondSVContract.getDPContract()).to.be.deep.equal(secondDPContract);
    expect(secondSVContract.getPreviousRevisions()).to.deep.equal([
      firstRevision,
    ]);
  });
});
