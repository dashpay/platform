const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  UpdateStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

const getStPacketFixture = require('../../../../../lib/test/fixtures/getStPacketFixture');
const getStHeaderFixture = require('../../../../../lib/test/fixtures/getStHeaderFixture');

use(chaiAsPromised);
use(dirtyChai);

// @TODO enable after js-dp-services-ctl will be fixed
describe.skip('updateStateHandlerFactory', function main() {
  this.timeout(160000);

  let removeDapi;
  let dapiClient;
  let stHeader;
  let stPacket;

  beforeEach(async () => {
    const {
      dapiCore,
      remove,
    } = await startDapi({
      dapi: {
        cacheNodeModules: true,
        localAppPath: process.cwd(),
        container: {
          volumes: [
            `${process.cwd()}/lib:/usr/src/app/lib`,
            `${process.cwd()}/scripts:/usr/src/app/scripts`,
          ],
        },
      },
      drive: {
        container: {
          image: 'drivewithnewapi',
        },
      },
      machine: {
        container: {
          image: 'abci',
        },
      },
    });

    dapiClient = dapiCore.getApi();
    removeDapi = remove;

    const stPacketFixture = getStPacketFixture();
    const stHeaderFixture = getStHeaderFixture(stPacketFixture);

    stHeader = Buffer.from(stHeaderFixture.serialize(), 'hex');
    stPacket = stPacketFixture.serialize();
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should respond with valid result', async () => {
    const result = await dapiClient.updateState(stHeader, stPacket);

    expect(result).to.be.an.instanceOf(UpdateStateTransitionResponse);

    // @TODO
    // getApi fetch documents
    // getContracts
    // check them
  });
});
