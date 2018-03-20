const { Message } = require('bitcore-lib-dash');
const Mnemonic = require('bitcore-mnemonic-dash');
const registeredUser = require('../mocks/registeredUser');
const axios = require('axios');
const MNDiscoveryService = require('../../src/services/MNDiscoveryService');

async function explorerPost(apiMethod, data) {
  const MN = await MNDiscoveryService.getRandomMasternode();
  const uri = `http://${MN.host}:${MN.port}/${apiMethod}`;
  return axios.post(uri, data);
}

const mockUser = JSON.parse(registeredUser);
const data = {
  owner: 'Alice', receiver: 'Bob', type: 'contactReq', txId: mockUser.txid,
};

const mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
const privKey = mnemonic.toHDPrivateKey().derive('m/1').privateKey;
const signature = Message(JSON.stringify(data)).sign(privKey);

explorerPost('/quorum', {
  verb: 'add',
  qid: 0,
  data,
  signature,
});

// Override node promises (workaround debug issues)
global.Promise = require('bluebird');

// new Promise((resolve, reject) => {
//     breaksomething() //won't pause
// })
