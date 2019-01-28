const MNDiscovery = require('../../../src/MNDiscovery/index');
const sinon = require('sinon');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const RPCClient = require('../../../src/RPCClient');
const config = require('../../../src/config');

chai.use(chaiAsPromised);
const {expect} = chai;

const MockedMNList = [{
    vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
    status: 'ENABLED',
    rank: 1,
    ip: '138.156.10.21',
    protocol: 70208,
    payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
    activeseconds: 1073078,
    lastseen: 1516291362,
}, {
    vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
    status: 'ENABLED',
    rank: 1,
    ip: '171.86.98.52',
    protocol: 70208,
    payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
    activeseconds: 1073078,
    lastseen: 1516291362,
}, {
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

describe('MNDiscovery', async () => {

    describe('.getMNList()', async () => {

        before(() => {
            // Stub for request to seed, which is 127.0.0.1
            const RPCClientStub = sinon.stub(RPCClient, 'request');
            RPCClientStub
                .withArgs({host: '127.0.0.1', port: config.Api.port}, 'getMNList', {})
                .returns(new Promise((resolve) => {
                    resolve(MockedMNList);
                }));
        });

        after(() => {
            RPCClient.request.restore();
        });

        it('Should return MN list', async () => {
            const discovery = new MNDiscovery();
            sinon.spy(discovery.masternodeListProvider, 'getMNList');

            const MNList = await discovery.getMNList();

            var i = 0;
            MNList.forEach((MNListItem) => {
                expect(MNListItem);
                expect(MNListItem.ip).to.be.equal(MockedMNList[i].ip);
                expect(MNListItem.status).to.be.a('string');
                expect(MNListItem.rank).to.be.a('number');
                expect(MNListItem.lastseen).to.be.a('number');
                expect(MNListItem.activeseconds).to.be.a('number');
                expect(discovery.masternodeListProvider.getMNList.callCount).to.equal(1);
                i++;
            });

            discovery.masternodeListProvider.getMNList.restore();
        });

        it('Should reset cached MNList and resets it back to initial seed', async () => {
            const discovery = new MNDiscovery();
            sinon.spy(discovery.masternodeListProvider, 'getMNList');
            discovery.reset();
            expect(discovery.masternodeListProvider.seeds).to.be.undefined;
        });

        it('Should return random node from MN list', async () => {
            const discovery = new MNDiscovery();
            sinon.spy(discovery.masternodeListProvider, 'getMNList');

            var ips = [];
            async function verifyRandomf(array) {
                for (const item of array) {
                    let randomMasternode = await discovery.getRandomMasternode();
                    expect(masternodeIps).to.contain(randomMasternode.ip);
                    expect(randomMasternode.ip).to.be.a('string');
                    expect(randomMasternode.status).to.be.a('string');
                    expect(randomMasternode.rank).to.be.a('number');
                    expect(randomMasternode.lastseen).to.be.a('number');
                    expect(randomMasternode.activeseconds).to.be.a('number');
                    ips.push(randomMasternode.ip);
                }
                expect(discovery.masternodeListProvider.getMNList.callCount).to.equal(array.length);
                let uniqueIps = ips.filter(function (elem, pos) {
                    return ips.indexOf(elem) == pos;
                });
                expect(uniqueIps.length > 1).to.be.true;
                discovery.masternodeListProvider.getMNList.restore();
            }

            await verifyRandomf([0, 1, 2, 3]);
        });
    });

});
