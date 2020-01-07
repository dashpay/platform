const DashJS = require('dash');
const sdkOpts = {
  network: 'testnet',
  mnemonic: null,// Will generate a new address, you should keep it.
};
const sdk = new DashJS.SDK(sdkOpts);

const displayFundingAddress = async function () {
  const {account, wallet} = sdk;

  const mnemonic = wallet.exportWallet();
  const address = account.getUnusedAddress().address;
  console.log('Mnemonic:', mnemonic);
  console.log('Total balance',  account.getTotalBalance());
  console.log('Unused address:', address);
  // Fund this address using the faucet : http://devnet-evonet-1117662964.us-west-2.elb.amazonaws.com/
};
const onReceivedTransaction = function(data){
  const {account} = sdk;
  console.log('Received tx',data.txid);
  console.log('Total pending confirmation',  account.getUnconfirmedBalance());
  console.log('Total balance',  account.getTotalBalance());
}
sdk.account.events.on('FETCHED/UNCONFIRMED_TRANSACTION',onReceivedTransaction)
sdk.isReady().then(displayFundingAddress);

