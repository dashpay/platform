const DapObject = require('../../../../lib/stateView/dapObject/DapObject');
const Reference = require('../../../../lib/stateView/Reference');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const startMongoDbInstance = require('../../../../lib/test/services/mocha/startMongoDbInstance');

describe('DapObjectMongoDbRepository', () => {
  let dapObjectRepository;
  startMongoDbInstance().then(async (mongoDbInstance) => {
    const mongoClient = await mongoDbInstance.mongoClient;
    const mongoDb = mongoClient.db('test_dap');
    dapObjectRepository = new DapObjectMongoDbRepository(mongoDb);
  });

  it('should store DapObject entity', async () => {
    const id = '123456';
    const objectData = {
      id,
      act: 0,
      objtype: 'DashPayContact',
      rev: 0,
    };
    const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const blockHeight = 1;
    const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
    const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
    const reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
    );
    const dapObject = new DapObject(objectData, reference);

    await dapObjectRepository.store(dapObject);
    const object = await dapObjectRepository.find(id);
    expect(object.toJSON()).to.deep.equal(dapObject.toJSON());
  });

  it('should delete DapObject entity', async () => {
    const id = '123456';
    const objectData = {
      id,
      act: 0,
      objtype: 'DashPayContact',
      rev: 0,
    };
    const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const blockHeight = 1;
    const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
    const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
    const reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
    );
    const dapObject = new DapObject(objectData, reference);

    await dapObjectRepository.store(dapObject);
    const object = await dapObjectRepository.find(id);
    expect(object.toJSON()).to.deep.equal(dapObject.toJSON());

    await dapObjectRepository.delete(dapObject);
    const objectTwo = await dapObjectRepository.find(id);
    const serializeObject = objectTwo.toJSON();
    expect(serializeObject.id).to.not.exist();
  });

  it('should return empty DapObject if not found', async () => {
    const object = await dapObjectRepository.find();

    const serializeObject = object.toJSON();
    expect(serializeObject.id).to.not.exist();
  });
});
