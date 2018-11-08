const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');
const generateDapObjectId = require('../../../../lib/stateView/dapObject/generateDapObjectId');
const Reference = require('../../../../lib/stateView/Reference');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const updateDapObjectFactory = require('../../../../lib/stateView/dapObject/updateDapObjectFactory');

describe('updateDapObjectFactory', () => {
  const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
  const blockHeight = 1;
  const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
  const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
  const objectHash = '123981as90d01309ad09123';

  let mongoClient;
  startMongoDb().then(async (mongoDbInstance) => {
    mongoClient = await mongoDbInstance.getClient();
  });

  let updateDapObject;
  let reference;

  const dapId = '1234';
  const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';

  let createDapObjectMongoDbRepository;
  beforeEach(() => {
    createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
    updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);

    reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
      objectHash,
    );
  });

  it('should store DapObject if action is 0', async () => {
    const dapObjectData = {
      objtype: 'user',
      pver: 1,
      idx: 0,
      rev: 1,
      act: 0,
    };
    await updateDapObject(dapId, blockchainUserId, reference, dapObjectData);

    const dapObjectMongoDbRepository = createDapObjectMongoDbRepository(dapId);
    const id = generateDapObjectId(blockchainUserId, dapObjectData.idx);
    const dapObject = await dapObjectMongoDbRepository.find(id);
    expect(dapObject.getOriginalData()).to.be.deep.equal(dapObjectData);
  });

  it('should store DapObject with revisions if action is 1', async () => {
    const revisionOneDapObjectData = {
      objtype: 'user',
      pver: 1,
      idx: 0,
      rev: 0,
      act: 0,
    };
    await updateDapObject(dapId, blockchainUserId, reference, revisionOneDapObjectData);
    const revisionTwoDapObjectData = {
      objtype: 'user',
      pver: 1,
      idx: 0,
      rev: 1,
      act: 1,
    };
    await updateDapObject(dapId, blockchainUserId, reference, revisionTwoDapObjectData);

    const dapObjectMongoDbRepository = createDapObjectMongoDbRepository(dapId);
    const id = generateDapObjectId(blockchainUserId, revisionOneDapObjectData.idx);
    const dapObject = await dapObjectMongoDbRepository.find(id);
    expect(dapObject.getRevision()).to.be.equal(revisionTwoDapObjectData.rev);
    expect(dapObject.getPreviousRevisions()).to.be.deep.equal([
      {
        revision: revisionOneDapObjectData.rev,
        reference,
      },
    ]);
  });

  it('should mark DapObject as deleted if action is 2', async () => {
    const dapObjectData = {
      objtype: 'user',
      pver: 1,
      idx: 0,
      rev: 0,
      act: 2,
    };
    await updateDapObject(dapId, blockchainUserId, reference, dapObjectData);

    const dapObjectMongoDbRepository = createDapObjectMongoDbRepository(dapId);
    const id = generateDapObjectId(blockchainUserId, dapObjectData.idx);
    const dapObject = await dapObjectMongoDbRepository.find(id);
    expect(dapObject.isDeleted()).to.be.true();
  });
});
