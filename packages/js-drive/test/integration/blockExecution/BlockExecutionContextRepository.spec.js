const rimraf = require('rimraf');
const cbor = require('cbor');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const Drive = require('@dashevo/rs-drive');
const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const noopLogger = require('../../../lib/util/noopLogger');
const BlockExecutionContextRepository = require('../../../lib/blockExecution/BlockExecutionContextRepository');

describe('BlockExecutionContextRepository', () => {
  // let container;
  let blockExecutionContextRepository;
  let blockExecutionContext;
  let rsDrive;
  let store;
  let options;

  beforeEach(async () => {
    const dataContract = getDataContractFixture();
    delete dataContract.entropy;

    const plainObject = getBlockExecutionContextObjectFixture(dataContract);

    blockExecutionContext = new BlockExecutionContext();
    blockExecutionContext.fromObject(plainObject);

    rsDrive = new Drive('./db/grovedb_test', {
      dataContractsGlobalCacheSize: 500,
      dataContractsTransactionalCacheSize: 500,
    });

    store = new GroveDBStore(rsDrive, noopLogger);

    blockExecutionContextRepository = new BlockExecutionContextRepository(store);

    options = {};
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });

  it('should store blockExecutionContext', async () => {
    const result = await blockExecutionContextRepository.store(blockExecutionContext, options);

    expect(result).to.be.instanceOf(BlockExecutionContextRepository);

    const encodedResult = await store.getAux(
      BlockExecutionContextRepository.EXTERNAL_STORE_KEY_NAME,
      options,
    );

    const blockExecutionContextEncoded = encodedResult.getValue();

    const rawBlockExecutionContext = cbor.decode(blockExecutionContextEncoded);

    expect(rawBlockExecutionContext).to.deep.equal(blockExecutionContext.toObject({
      skipConsensusLogger: true,
      skipTransaction: true,
    }));
  });

  it('should fetch blockExecutionContext', async () => {
    await store.putAux(
      BlockExecutionContextRepository.EXTERNAL_STORE_KEY_NAME,
      await cbor.encodeAsync(blockExecutionContext.toObject({
        skipConsensusLogger: true,
      })),
      options,
    );

    const fetchedBlockExecutionContext = await blockExecutionContextRepository.fetch(options);

    expect(fetchedBlockExecutionContext).to.be.instanceOf(BlockExecutionContext);

    expect(fetchedBlockExecutionContext.toObject({
      skipConsensusLogger: true,
    })).to.deep.equal(blockExecutionContext.toObject({
      skipConsensusLogger: true,
    }));
  });

  it('should fetch blockExecutionContext stored in transaction', async () => {
    await store.startTransaction();

    await blockExecutionContextRepository.store(blockExecutionContext, { useTransaction: true });

    let fetchedBlockExecutionContext = await blockExecutionContextRepository.fetch();

    expect(fetchedBlockExecutionContext).to.be.instanceOf(BlockExecutionContext);

    expect(fetchedBlockExecutionContext.isEmpty()).to.be.true();

    await store.commitTransaction();

    fetchedBlockExecutionContext = await blockExecutionContextRepository.fetch();

    expect(fetchedBlockExecutionContext).to.be.instanceOf(BlockExecutionContext);

    expect(fetchedBlockExecutionContext.toObject({
      skipConsensusLogger: true,
    })).to.deep.equal(blockExecutionContext.toObject({
      skipConsensusLogger: true,
    }));
  });
});
