const MNListProvider = require('../../../src/MNDiscovery/MasternodeListProvider');
const sinon = require('sinon');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const RPCClient = require('../../../src/RPCClient');
const config = require('../../../src/config');

chai.use(chaiAsPromised);
const { expect } = chai;

const MockedMNList = [{
  vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
  status: 'ENABLED',
  rank: 1,
  ip: '138.156.10.21',
  protocol: 70208,
  payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
  activeseconds: 1073078,
  lastseen: 1516291362,
},{
  vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
  status: 'ENABLED',
  rank: 1,
  ip: '171.86.98.52',
  protocol: 70208,
  payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
  activeseconds: 1073078,
  lastseen: 1516291362,
},{
  vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
  status: 'ENABLED',
  rank: 1,
  ip: '146.81.95.64',
  protocol: 70208,
  payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
  activeseconds: 1073078,
  lastseen: 1516291362,
}];

const updatedMNList = [{
  vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
  status: 'ENABLED',
  rank: 1,
  ip: '149.80.91.62',
  protocol: 70208,
  payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
  activeseconds: 1073078,
  lastseen: 1516291362,
}];

const masternodesThatReturnEmptyList = [{
  vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
  status: 'ENABLED',
  rank: 1,
  ip: '128.01.01.01',
  protocol: 70208,
  payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
  activeseconds: 1073078,
  lastseen: 1516291362,
}];

const masternodesThatReturnNull = [{
  vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
  status: 'ENABLED',
  rank: 1,
  ip: '129.02.02.02',
  protocol: 70208,
  payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
  activeseconds: 1073078,
  lastseen: 1516291362,
}];

describe('MNListProvider', async () => {

  describe('.getMNList()', async () => {

    before(() => {
      // Stub for request to seed, which is 127.0.0.1
      const RPCClientStub = sinon.stub(RPCClient, 'request');
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMNList', {})
        .returns(new Promise((resolve) => {
          resolve(MockedMNList);
        }));
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port +1 }, 'getMNList', {})
        .returns(new Promise((resolve) => {
          resolve(MockedMNList);
        }));

      // Stubs for request to any MN from MNList, returned by seed. This call should return updated list
      for (let masternode of MockedMNList) {
        RPCClientStub
          .withArgs({ host: masternode.ip, port: config.Api.port }, 'getMNList', {})
          .returns(new Promise((resolve) => {
            resolve(updatedMNList);
          }));
      }

      // Stubs for request to masternodes that should return empty list
      for (let masternode of masternodesThatReturnEmptyList) {
        RPCClientStub
          .withArgs({ host: masternode.ip, port: config.Api.port }, 'getMNList', {})
          .returns(new Promise((resolve) => {
            resolve([]);
          }));
      }

      // Stubs for request to masternodes that should return null
      for (let masternode of masternodesThatReturnNull) {
        RPCClientStub
          .withArgs({ host: masternode.ip, port: config.Api.port }, 'getMNList', {})
          .returns(new Promise((resolve) => {
            resolve(null);
          }));
      }
    });

    after(() => {
      RPCClient.request.restore();
    });

    it('Should fetch MN list from seed if list has never updated', async() => {
      const mnListProvider = new MNListProvider();
      expect(mnListProvider.lastUpdateDate).to.equal(0);
      expect(mnListProvider.masternodeList.length).to.equal(1);
      expect(mnListProvider.masternodeList[0].ip).to.equal(config.DAPIDNSSeeds[0].ip);

      const MNList = await mnListProvider.getMNList();
      const MNListItem = MNList[0];

      expect(MNListItem.ip).to.be.a('string');
      expect(MNListItem.status).to.be.a('string');
      expect(MNListItem.rank).to.be.a('number');
      expect(MNListItem.lastseen).to.be.a('number');
      expect(MNListItem.activeseconds).to.be.a('number');
      expect(MNListItem.payee).to.be.a('string');
      expect(MNListItem.protocol).to.be.a('number');
      expect(MNListItem.rank).to.be.a('number');
      expect(MNListItem.vin).to.be.a('string');

      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(3);
    });
    it('Should update MNList if needed and return updated list', async () => {
      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(3);
      let MNListItem = MNList[0];
      expect(MNListItem.ip).to.be.equal(MockedMNList[0].ip);

      // Set time, so update MNList is required
      mnListProvider.lastUpdateDate -= config.MNListUpdateInterval * 2;

      MNList = await mnListProvider.getMNList();
      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(1);
      expect(MNList).to.be.an('array');
      MNListItem = MNList[0];
      expect(MNListItem.ip).to.be.equal(updatedMNList[0].ip);
    });
    it('Should not update MNList if no update needed and return cached list', async () => {
      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      let MNListItem = MNList[0];
      const updateDate = mnListProvider.lastUpdateDate;
      const networkCallCount = RPCClient.request.callCount;

      expect(mnListProvider.lastUpdateDate).be.closeTo(Date.now(), 10000);
      expect(mnListProvider.masternodeList.length).to.equal(3);
      expect(MNListItem.ip).to.be.equal(MockedMNList[0].ip);

      // Call getMNList one more time right after update; Expect results to be the same, and update date not changed
      // Also no network call should be done

      MNList = await mnListProvider.getMNList();
      MNListItem = MNList[0];

      expect(mnListProvider.lastUpdateDate).be.equal(updateDate);
      expect(mnListProvider.masternodeList.length).to.equal(3);
      expect(MNListItem.ip).to.be.equal(MockedMNList[0].ip);
      expect(RPCClient.request.callCount).to.be.equal(networkCallCount);
    });
    it('Should throw error if can\'t connect to dns seeder', async () => {
      // Override stub behaviour for next call
      RPCClient.request.resetHistory();
      RPCClient.request
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMNList', {})
        .onFirstCall()
        .returns(new Promise(resolve => resolve(null)));

      const mnListProvider = new MNListProvider();
      return expect(mnListProvider.getMNList()).to.be.rejectedWith('Failed to fetch masternodes list');
    });
    it('Should throw error if can\'t connect to dns seeder, wrong port', async () => {
        // Override stub behaviour for next call
        RPCClient.request.resetHistory();
        RPCClient.request
          .withArgs({ host: '127.0.0.1', port: config.Api.port+1 }, 'getMNList', {})
          .onFirstCall()
          .returns(new Promise(resolve => resolve(null)));

        const mnListProvider = new MNListProvider();
        return expect(mnListProvider.getMNList()).to.be.rejectedWith('Failed to fetch masternodes list');
      });
    it('Should throw error if can\'t update masternode list', async () => {
      // Override stub behaviour for next call
      RPCClient.request.resetHistory();
      RPCClient.request
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMNList', {})
        .onFirstCall()
        .returns(new Promise(resolve => resolve(masternodesThatReturnNull)));

      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      expect(mnListProvider.masternodeList.length).to.equal(1);
      expect(MNList.length).to.equal(1);

      // Adjust time for update
      mnListProvider.lastUpdateDate -= config.MNListUpdateInterval * 2;

      return expect(mnListProvider.getMNList()).to.be.rejectedWith('Failed to fetch masternodes list');
    });
    it('Should not update list if fetched list is empty', async () => {
      // Override stub behaviour for next call
      RPCClient.request.resetHistory();
      RPCClient.request
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMNList', {})
        .onFirstCall()
        .returns(new Promise(resolve => resolve(masternodesThatReturnEmptyList)));

      const mnListProvider = new MNListProvider();
      let MNList = await mnListProvider.getMNList();
      expect(mnListProvider.masternodeList.length).to.equal(1);
      expect(MNList.length).to.equal(1);

      // Adjust time for update
      mnListProvider.lastUpdateDate -= config.MNListUpdateInterval * 2;

      MNList = await mnListProvider.getMNList();
      expect(MNList[0].ip).to.equal(masternodesThatReturnEmptyList[0].ip);
    });
  });

});