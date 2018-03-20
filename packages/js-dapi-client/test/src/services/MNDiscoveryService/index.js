const MNDiscoveryService = require('../../../../src/services/MNDiscoveryService/index');
const MNListProvider = require('../../../../src/services/MNDiscoveryService/MasternodeListProvider');
const sinon = require('sinon');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const RPCClient = require('../../../../src/utils/RPCClient');
const config = require('../../../../src/config/index');

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

const masternodeIps = MockedMNList.map(masternode => masternode.ip);

describe('MNDiscoveryService', async () => {

  describe('.getMNList()', async () => {

    before(() => {
      // Stub for request to seed, which is 127.0.0.1
      const RPCClientStub = sinon.stub(RPCClient, 'request');
      RPCClientStub
        .withArgs({ host: '127.0.0.1', port: config.Api.port }, 'getMNList', [])
        .returns(new Promise((resolve) => {
          resolve(MockedMNList);
        }));
    });

    beforeEach(() => {
      // Reset cached MNList
      MNDiscoveryService.reset();
      sinon.spy(MNDiscoveryService.masternodeListProvider, 'getMNList');
    });

    afterEach(()=> {
      MNDiscoveryService.masternodeListProvider.getMNList.restore();
    });

    after(() => {
      MNDiscoveryService.reset();
      RPCClient.request.restore();
    });

    it('Should return MN list', async() => {
      const MNList = await MNDiscoveryService.getMNList();
      const MNListItem = MNList[0];

      expect(MNListItem);
      expect(MNListItem.ip).to.be.equal(MockedMNList[0].ip);
      expect(MNListItem.status).to.be.a('string');
      expect(MNListItem.rank).to.be.a('number');
      expect(MNListItem.lastseen).to.be.a('number');
      expect(MNListItem.activeseconds).to.be.a('number');
      expect(MNDiscoveryService.masternodeListProvider.getMNList.callCount).to.equal(1);

    });

    it('Should return random node from MN list', async() => {
      const randomMasternode = await MNDiscoveryService.getRandomMasternode();

      expect(masternodeIps).to.contain(randomMasternode.ip);
      expect(randomMasternode.ip).to.be.a('string');
      expect(randomMasternode.status).to.be.a('string');
      expect(randomMasternode.rank).to.be.a('number');
      expect(randomMasternode.lastseen).to.be.a('number');
      expect(randomMasternode.activeseconds).to.be.a('number');
      expect(MNDiscoveryService.masternodeListProvider.getMNList.callCount).to.equal(1);
    });
  });

});