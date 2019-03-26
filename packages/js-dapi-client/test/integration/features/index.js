require('../../bootstrap');

const path = require('path');
const dotenvSafe = require('dotenv-safe');

const sinon = require('sinon');

const MNDiscovery = require('../../../src/MNDiscovery/index');
const { startDapi } = require('@dashevo/dp-services-ctl');
const DAPIClient = require('../../../src/index');


const {
  PrivateKey,
  PublicKey,
  Address,
} = require('@dashevo/dashcore-lib');

const wait = require('../../utils/wait');

process.env.NODE_ENV = 'test';

dotenvSafe.config({
  sample: path.resolve(__dirname, '../.env'),
  path: path.resolve(__dirname, '../.env'),
});


describe('retry policy', () => {
  let masterNode;
  let seeds;

  let spy;
  let spy2;

  let transactionIdSendToAddress;
  let insightURL;

  let dapiClient;
  let dapId;
  let dapSchema;
  let dapContract;

  let faucetPrivateKey;
  let faucetAddress;

  let bobUserName;

  before(async () => {
    const privKey = 'cVwyvFt95dzwEqYCLd8pv9CzktajP4tWH2w9RQNPeHYA7pH35wcJ';
    faucetPrivateKey = new PrivateKey(privKey);

    const faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);

    faucetAddress = Address
      .fromPublicKey(faucetPublicKey, 'testnet')
      .toString();

    bobUserName = Math.random()
      .toString(36)
      .substring(7);
    aliceUserName = Math.random()
      .toString(36)
      .substring(7);

    const dpContract = dpp.contract.create(entropy.generate(), {
      user: {
        properties: {
          avatarUrl: {
            type: 'string',
            format: 'url',
          },
          about: {
            type: 'string',
          },
        },
        required: ['avatarUrl', 'about'],
        additionalProperties: false,
      },
      contact: {
        properties: {
          toUserId: {
            type: 'string',
          },
          publicKey: {
            type: 'string',
          },
        },
        required: ['toUserId', 'publicKey'],
        additionalProperties: false,
      },
    });

    dpp.setDPContract(dpContract);

    sinon.stub(MNDiscovery.prototype, 'getRandomMasternode')
      .returns(Promise.resolve({ ip: '127.0.0.1' }));

    [masterNode] = await startDapi.many(1);

    const seeds = [{ ip: masterNode.dapi.container.getIp() }];
    await masterNode.dashCore.getApi()
      .generate(1500);

    dapiClient = new DAPIClient({
      seeds,
      port: masterNode.dapi.options.getRpcPort(),
    });

    insightURL = `http://127.0.0.1:${masterNode.insight.options.getApiPort()}/insight-api-dash`;

    transactionIdSendToAddress = await masterNode.dashCore.getApi()
      .sendToAddress(faucetAddress, 100);
    await dapiClient.generate(20);
    let result = await masterNode.dashCore.getApi()
      .getAddressUtxos({ 'addresses': ['ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh'] });
    await wait(20000);
    spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
    spy2 = sinon.spy(dapiClient, 'makeRequest');

  });

  after('cleanup lone services', async () => {
    const instances = [
      masterNode,
    ];

    await Promise.all(instances.filter(i => i)
      .map(i => i.remove()));

    MNDiscovery.prototype.getRandomMasternode.restore();
  });


  describe('dapi unavailable', () => {
    before(async () => {
      await masterNode.dapi.container.stop();
    });

    after(async () => {
      await masterNode.dapi.container.start();
      await wait(20000);
    });

    it('should makeRequestWithRetries be called 4 times with default settings', async function it() {
      let err = '';
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(4);

    });

    it('should makeRequestToRandomDAPINode be called 1 time with default settings', async function it() {
      let err = '';
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
      });
      let spy = sinon.spy(dapiClient, 'makeRequestToRandomDAPINode');
      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(1);

    });

    it('should rpc method be called 1 time with default settings', async function it() {
      let err = '';
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
      });
      let spy = sinon.spy(dapiClient, 'getBestBlockHeight');
      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(1);

    });

    it('should makeRequestWithRetries be called 11 times with retries=10', async function it() {
      let err = '';
      const retries = 10;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(11);

    });

    it('should makeRequestWithRetries be called 1 times with retries=0', async function it() {
      let err = '';
      const retries = 0;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(4);

    });

    it('should makeRequestWithRetries be called 1 times with retries=true', async function it() {
      const retries = true;
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect retries to be an unsigned integer');
    });

    it('should makeRequestWithRetries be called 1 times with retries=1', async function it() {
      let err = '';
      const retries = 1;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(2);

    });

    it('should makeRequestWithRetries be called 1 times with retries=-10', async function it() {
      const retries = -10;
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect retries to be an unsigned integer');
    });

    it('should makeRequestWithRetries be called 1 times with retries=str', async function it() {
      const retries = 'str';
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect retries to be an unsigned integer');
    });

    it('should DAPIClient throw error when timeout=str', async function it() {
      const timeout = 'str';
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect timeout to be an unsigned integer');
    });

    it('should be able to use integer as string for timeout parameter', async function it() {
      const timeout = '100';
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect timeout to be an unsigned integer');
    });

    it('should be able to use integer for timeout parameter', async function it() {
      let err = '';
      const timeout = 100000;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');

      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(4);
    });

    it('should be able to use timeout parameter with min value=1', async function it() {
      let err = '';
      const timeout = 1;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');

      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(4);
    });

    it('should DAPIClient throw error when timeout=-1', async function it() {
      const timeout = -1;
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect timeout to be an unsigned integer');
    });

    it('should DAPIClient throw error when timeout=true', async function it() {
      const timeout = true;
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect timeout to be an unsigned integer');
    });

    it('should DAPIClient throw error when timeout="100"', async function it() {
      const timeout = '100';
      return expect(() => new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      }))
        .to
        .throw(Error, 'Invalid Argument: Expect timeout to be an unsigned integer');
    });
  });
  describe('dapi started', () => {
    it('should makeRequestWithRetries be called 1 times with default settings', async function it() {
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      await dapiClient.getBestBlockHeight();
      expect(spy.callCount)
        .to
        .be
        .equal(1);
    });

    it('should makeRequestToRandomDAPINode be called 0 times with default settings', async function it() {
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
      });
      let spy = sinon.spy(dapiClient, 'makeRequestToRandomDAPINode');
      expect(spy.callCount)
        .to
        .be
        .equal(0);
    });

    it('should getBestBlockHeight be called 1 times with default settings', async function it() {
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
      });
      let spy = sinon.spy(dapiClient, 'getBestBlockHeight');
      await dapiClient.getBestBlockHeight();
      expect(spy.callCount)
        .to
        .be
        .equal(1);
    });

    it('should makeRequestWithRetries be called 1 times with retries=10', async function it() {
      const retries = 10;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      await dapiClient.getBestBlockHeight();
      expect(spy.callCount)
        .to
        .be
        .equal(1);
    });

    it('should makeRequestWithRetries be called 1 times with retries=0', async function it() {
      const retries = 0;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      await dapiClient.getBestBlockHeight();

      expect(spy.callCount)
        .to
        .be
        .equal(1);
    });

    it('should makeRequestWithRetries be called 1 times with retries=1', async function it() {
      const retries = 1;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        retries: retries
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      await dapiClient.getBestBlockHeight();
      expect(spy.callCount)
        .to
        .be
        .equal(1);
    });

    it('should makeRequestWithRetries be called 1 times with timeout=10000', async function it() {
      const timeout = 10000;
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');
      await dapiClient.getBestBlockHeight();

      expect(spy.callCount)
        .to
        .be
        .equal(1);
    });

    it('should DAPIClient throw error when timeout too small', async function it() {
      const timeout = 1;
      let err = '';
      dapiClient = new DAPIClient({
        seeds,
        port: masterNode.dapi.options.getRpcPort(),
        timeout: timeout
      });
      let spy = sinon.spy(dapiClient, 'makeRequestWithRetries');

      try {
        await dapiClient.getBestBlockHeight();
      } catch (e) {
        err = e;
      }
      expect(err.message)
        .to
        .equal('max retries to connect to DAPI node reached');
      expect(spy.callCount)
        .to
        .be
        .equal(4);
    });
  });

});

