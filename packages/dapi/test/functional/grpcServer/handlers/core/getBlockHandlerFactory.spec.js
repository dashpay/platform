const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const { Block } = require('@dashevo/dashcore-lib');

describe('getBlockHandlerFactory', function main() {
  this.timeout(400000);

  let removeDapi;
  let dapiClient;
  let blockHash;
  let blockHeight;

  beforeEach(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    const coreAPI = dashCore.getApi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();

    const { result: address } = await coreAPI.getNewAddress();

    await dashCore.getApi().generateToAddress(101, address);

    ({ result: blockHash } = await dashCore.getApi().getbestblockhash());
    blockHeight = 100;
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should get block by hash', async () => {
    const blockBinary = await dapiClient.getBlockByHash(blockHash);

    expect(blockBinary).to.be.an.instanceof(Buffer);
    const block = new Block(blockBinary);

    expect(block.toBuffer()).to.deep.equal(blockBinary);
  });

  it('should get block by height', async () => {
    const blockBinary = await dapiClient.getBlockByHeight(blockHeight);

    expect(blockBinary).to.be.an.instanceof(Buffer);
    const block = new Block(blockBinary);

    expect(block.toBuffer()).to.deep.equal(blockBinary);
  });

  it('should respond with an invalid argument error if the block was not found', async () => {
    try {
      await dapiClient.getBlockByHeight(100000000);

      expect.fail('Should throw an invalid argument error');
    } catch (e) {
      expect(e.message).to.equal('3 INVALID_ARGUMENT: Invalid argument: Invalid block height');
      expect(e.code).to.equal(3);
    }
  });
});
