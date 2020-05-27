const network = "testnet";
const opts = {
  network,
  wallet: {
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
  },
  apps: {
    dashpay: {
    contractId: ''// Provide the dashpay contract id here
    }
  }
};
const client = new Dash.Client(opts);
client.isReady().then(()=>{
  const {account, platform} = client;

  async function sendPayment() {
    const tx = await account.createTransaction({recipient: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf', satoshis: 12000});
    console.log(await account.broadcastTransaction(tx));
  }

  async function readDocument() {
    const profile = await platform.documents.fetch('dashpay.profile', {});
    console.log(profile);
  }
});
