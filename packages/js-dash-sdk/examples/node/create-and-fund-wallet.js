const Dash = require('dash');
const clientOpts = {
  network: 'testnet',
  mnemonic: null,// Will generate a new address, you should keep it.
};
const client = new Dash.Client(clientOpts);

const displayFundingAddress = async function () {
  const {account, wallet} = client;

  const mnemonic = wallet.exportWallet();
  const address = account.getUnusedAddress().address;
  console.log('Mnemonic:', mnemonic);
  console.log('Total balance',  account.getTotalBalance());
  console.log('Unused address:', address);
  // Fund this address using the faucet : http://devnet-evonet-1117662964.us-west-2.elb.amazonaws.com/
};
const onReceivedTransaction = function(data){
  const {account} = client;
  console.log('Received tx',data.txid);
  console.log('Total pending confirmation',  account.getUnconfirmedBalance());
  console.log('Total balance',  account.getTotalBalance());
}
client.account.on('FETCHED/UNCONFIRMED_TRANSACTION',onReceivedTransaction)
client.isReady().then(displayFundingAddress);

