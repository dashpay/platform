const { SimplifiedMNList } = require('@dashevo/dashcore-lib');
const MNListProvider = require('../../../src/MNDiscovery/MasternodeListProvider');
const sinon = require('sinon');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const RPCClient = require('../../../src/RPCClient');
const config = require('../../../src/config');
const SMNListFixture = require('../../fixtures/mnList');

const genesisHash = '00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c';
chai.use(chaiAsPromised);
const { expect } = chai;

const masternodesThatReturnNull = {
  "baseBlockHash": "0000000000000000000000000000000000000000000000000000000000000000",
  "blockHash": "0000000d3b44caf49b6bc84ece702874612aae9ff27cce6c12f04aaca9888e07",
  "cbTxMerkleTree": "0100000001f5c175b58c729c28d061c87e18499a3ce27db09cbc134b7d832f9851a920ee610101",
  "cbTx": "03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff1202bc1b0e2f5032506f6f6c2d74444153482fffffffff04b1475b96010000001976a9144f79c383bc5d3e9d4d81b98f87337cedfa78953688ac40c3609a010000001976a9149d3fd8118ca4e54aaaaa9a2ebd7956b72afbbae888ac8f7b0504000000001976a914badadfdebaa6d015a0299f23fbc1fcbdd72ba96f88ac00000000000000002a6a28a5aa058fabb30ba08c1c1a7519e16eeeb25541bae08213f5d67cc46ec90e94b7000000000200000000000000260100bc1b00003d138eeaa5f63b564835d4f0348ea57d97486cb835a084be919dbaf6ffa6ecb6",
  "deletedMNs": [
  ],
  "mnList": [
    {
      "proRegTxHash": "c48a44a9493eae641bea36992bc8c27eaaa33adb1884960f55cd259608d26d2f",
      "confirmedHash": "000000237725f8fe7d78153ae9c11193ee0cda18f8b48141acff8e1ac713da5b",
      "service": "173.61.30.231:19013",
      "pubKeyOperator": "8700add55a28ef22ec042a2f28e25fb4ef04b3024a7c56ad7eed4aebc736f312d18f355370dfb6a5fec9258f464b227e",
      "keyIDVoting": "fdb28c60e521e58cc8c5bd6582966eb654aa1e4d",
      "isValid": true
    }
  ],
  "merkleRootMNList": "b6eca6fff6ba9d91be84a035b86c48977da58e34f0d43548563bf6a5ea8e133d"
};

describe('MNListProvider', async () => {

  describe('.getMnListDiff()', async () => {

    before(() => {
      // Stub for request to seed, which is 127.0.0.1
      const RPCClientStub = sinon.stub(RPCClient, 'request');
      // let baseHash = config.nullHash;
      let baseHash = '00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c';
      let blockHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
      const genesisHeight = 0;
      const FirstBlockHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
      const SecondBaseHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
      const SecondBlockHash = '000000325235a2a92011589df3d2be404eaea062afb6fa9a0dc02eee6e53bec8';
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getBlockHash', { height: genesisHeight })
        .returns(new Promise((resolve) => {
          resolve(genesisHash);
        }));
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getBestBlockHash', {})
        .returns(new Promise((resolve) => {
          resolve(FirstBlockHash);
        }));
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMnListDiff', { baseBlockHash: baseHash, blockHash: blockHash })
        .returns(new Promise((resolve) => {
          resolve(SMNListFixture.getFirstDiff());
        }));
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMnListDiff', { baseBlockHash: SecondBaseHash, blockHash: SecondBlockHash })
        .returns(new Promise((resolve) => {
          resolve(SMNListFixture.getSecondDiff());
        }));
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port +1 }, 'getMnListDiff', {})
        .returns(new Promise((resolve) => {
          resolve(null);
        }));
      baseHash = SecondBaseHash;
      blockHash = SecondBlockHash;
      // Stubs for request to any MN from MNList, returned by seed. This call should return updated list
      for (let masternode of SMNListFixture.getFirstDiff().mnList) {
        RPCClientStub
          .withArgs({ host: masternode.service.split(':')[0], port: config.Api.port }, 'getMnListDiff', { baseBlockHash: baseHash, blockHash: blockHash })
          .returns(new Promise((resolve) => {
            resolve(SMNListFixture.getSecondDiff());
          }));
      }
      for (let masternode of SMNListFixture.getFirstDiff().mnList) {
        RPCClientStub
          .withArgs({ host: masternode.service.split(':')[0], port: config.Api.port }, 'getBestBlockHash', {} )
          .returns(new Promise((resolve) => {
            resolve(SecondBlockHash);
          }));
      }
    });

    after(() => {
      RPCClient.request.restore();
    });

    it('Should fetch MN list from seed if list has never updated', async() => {
      const mnListProvider = new MNListProvider();
      expect(mnListProvider.lastUpdateDate).to.equal(0);
      expect(mnListProvider.masternodeList.length).to.equal(0);
      expect(mnListProvider.seeds).to.be.deep.equal(config.DAPIDNSSeeds);

      const MNList = await mnListProvider.getMNList();
      const MNListItem = MNList[0];

      expect(MNListItem.proRegTxHash).to.be.a('string');
      expect(MNListItem.confirmedHash).to.be.a('string');
      expect(MNListItem.service).to.be.a('string');
      expect(MNListItem.pubKeyOperator).to.be.a('string');
      expect(MNListItem.keyIDVoting).to.be.a('string');
      expect(MNListItem.isValid).to.be.a('boolean');

      expect(mnListProvider.masternodeList.length).to.equal(114);
    });
    it('Should update MNList if needed and return updated list', async () => {
      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(114);
      let MNListItem = MNList[0];
      const smnList = new SimplifiedMNList(SMNListFixture.getFirstDiff());
      const SMNListFixtureItem = smnList.getValidMasternodesList()[0];
      expect(MNListItem.service).to.be.equal(SMNListFixtureItem.service);

      // Set time, so update MNList is required
      mnListProvider.lastUpdateDate -= config.MNListUpdateInterval * 2;

      MNList = await mnListProvider.getMNList();
      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(107);
      expect(MNList).to.be.an('array');
      MNListItem = MNList[0];
      smnList.applyDiff(SMNListFixture.getSecondDiff());
      const SMNListFixtureItem2 = smnList.getValidMasternodesList()[0];
      expect(MNListItem.service).to.be.equal(SMNListFixtureItem2.service);
    });
    it('Should not update MNList if no update needed and return cached list', async () => {
      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      let MNListItem = MNList[0];
      const updateDate = mnListProvider.lastUpdateDate;
      const networkCallCount = RPCClient.request.callCount;

      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(114);
      const smnList = new SimplifiedMNList(SMNListFixture.getFirstDiff());
      const SMNListFixtureItem = smnList.getValidMasternodesList()[0];
      expect(MNListItem.service).to.be.equal(SMNListFixtureItem.service);

      // Call getMNList one more time right after update; Expect results to be the same, and update date not changed
      // Also no network call should be done

      MNList = await mnListProvider.getMNList();
      MNListItem = MNList[0];

      expect(mnListProvider.lastUpdateDate).be.equal(updateDate);
      expect(mnListProvider.masternodeList.length).to.equal(114);
      expect(MNListItem.service).to.be.equal(SMNListFixtureItem.service);
      expect(RPCClient.request.callCount).to.be.equal(networkCallCount);
    });
    it('Should throw error if seed is not an array', async() => {
        return expect(() => new MNListProvider(1)).to.throw(Error, 'seed is not an array');
      });
    it('Should  throw error if seed is string', async() => {
          return expect(() => new MNListProvider("127.0.0.1")).to.throw(Error, 'seed is not an array');
      });
    it('Should throw error if can\'t connect to dns seeder', async () => {
      // Override stub behaviour for next call
      const baseHash = genesisHash;
      const blockHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
      RPCClient.request.resetHistory();
      RPCClient.request
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMnListDiff', { baseBlockHash: baseHash, blockHash: blockHash })
        .onFirstCall()
        .returns(new Promise(resolve => resolve(null)));

      const mnListProvider = new MNListProvider();
      return expect(mnListProvider.getMNList()).to.be.rejectedWith('Failed to get mn diff from node 127.0.0.1');
    });
    it('Should throw error if can\'t connect to dns seeder, wrong port', async () => {
        // Override stub behaviour for next call
      const baseHash = genesisHash;
      const blockHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
        RPCClient.request.resetHistory();
        RPCClient.request
          .withArgs({ host: '127.0.0.1', port: config.Api.port+1 }, 'getMnListDiff', { baseBlockHash: baseHash, blockHash: blockHash })
          .onFirstCall()
          .returns(new Promise(resolve => resolve(null)));

        const mnListProvider = new MNListProvider();
        return expect(mnListProvider.getMNList()).to.be.rejectedWith('Failed to get mn diff from node 127.0.0.1');
      });
    it('Should throw error if can\'t update masternode list', async () => {
      // Override stub behaviour for next call
      const baseHash = genesisHash;
      const blockHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
      RPCClient.request.resetHistory();
      RPCClient.request
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMnListDiff', { baseBlockHash: baseHash, blockHash: blockHash })
        .onFirstCall()
        .returns(new Promise(resolve => resolve(masternodesThatReturnNull)));

      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      expect(mnListProvider.masternodeList.length).to.equal(1);
      expect(MNList.length).to.equal(1);

      RPCClient.request
        .withArgs({ host: '173.61.30.231', port: 19013 }, 'getBestBlockHash', {})
        .returns(new Promise((resolve) => {
          resolve(null);
        }));
      // Adjust time for update
      mnListProvider.lastUpdateDate -= config.MNListUpdateInterval * 2;

      return expect(mnListProvider.getMNList()).to.be.rejectedWith('Failed to get mn diff from node 173.61.30.231');
    });
  });

});
