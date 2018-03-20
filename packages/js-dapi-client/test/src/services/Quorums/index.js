const { Message, PrivateKey } = require('bitcore-lib-dash');
const Mnemonic = require('bitcore-mnemonic-dash');
const should = require('should');

const mockUser = JSON.parse(require('../../../../poc/mocks/registeredUser'));

const _data = {
  owner: 'Alice', receiver: 'Bob', type: 'contactReq', txId: mockUser.txid,
};
const mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
const privKey = new PrivateKey(mnemonic.toHDPrivateKey().derive('m/1').privateKey.toString());
const invalidMnemonic = new Mnemonic('safe miss client congress mean vault barely wrestle grit cycle decade cycle');
const invalidPrivKey = invalidMnemonic.toHDPrivateKey().derive('m/1').privateKey;
const _signature = Message(JSON.stringify(_data)).sign(privKey);

const mockData = {
  verb: 'add',
  qid: 0,
  data: _data,
  signature: _signature,
};

// describe('Quorums', () => {
//   it('should post data to a valid quorum node', () => explorerPost('/quorum', mockData).then((res) => {
//     res.response.should.equal('Added');
//   }));
//
//   it('should fail for posting with signature from incorrect private key', () => {
//     mockData.signature = message(JSON.stringify(_data)).sign(invalidPrivKey);
//
//     return explorerPost('/quorum', mockData).then((res) => {
//       res.response.should.equal('Failed');
//     });
//   });
//
//   it('should fail for posting to invalid quorum node', () => {
//     const nonQuorumNodes = SDK.Discover.Masternode.masternodeList.nodes.filter(x => SDK.Discover.Masternode.candidateList.indexOf(x) == -1);
//
//     SDK.Discover.Masternode.candidateList = nonQuorumNodes;
//
//     return explorerPost('/quorum', mockData).then((res) => {
//       res.response.should.equal('Failed');
//     });
//   });
// });
