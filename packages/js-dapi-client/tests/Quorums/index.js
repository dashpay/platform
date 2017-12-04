require('../_before.js');
const explorerPost = require('../../Common/ExplorerHelper').explorerPost;
const message = require('bitcore-message-dash');
const Mnemonic = require('bitcore-mnemonic-dash');
const should = require('should')

var mockUser = JSON.parse(require('../../Accounts/User/mocks/registeredUser'));
var _data = { owner: 'Alice', receiver: 'Bob', type: 'contactReq', txId: mockUser.txid }
var mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
var privKey = mnemonic.toHDPrivateKey().derive("m/1").privateKey;
var invalidMnemonic = new Mnemonic('safe miss client congress mean vault barely wrestle grit cycle decade cycle');
var invalidPrivKey = invalidMnemonic.toHDPrivateKey().derive("m/1").privateKey;
var _signature = message(JSON.stringify(_data)).sign(privKey);

var mockData = {
    verb: 'add',
    qid: 0,
    data: _data,
    signature: _signature
}

describe('Quorums', function() {


    it('should post data to a valid quorum node', function() {
        return explorerPost(`/quorum`, mockData).then(res => {
            res.response.should.equal('Added')
        })
    });

    it('should fail for posting with signature from incorrect private key', function() {
        mockData.signature = message(JSON.stringify(_data)).sign(invalidPrivKey)

        return explorerPost(`/quorum`, mockData).then(res => {
            res.response.should.equal('Failed')
        })
    });

    it('should fail for posting to invalid quorum node', function() {

        let nonQuorumNodes = SDK.Discover.Masternode.masternodeList.nodes.filter(x => {
            return SDK.Discover.Masternode.candidateList.indexOf(x) == -1
        })

        SDK.Discover.Masternode.candidateList = nonQuorumNodes

        return explorerPost(`/quorum`, mockData).then(res => {
            res.response.should.equal('Failed')
        })
    });

});