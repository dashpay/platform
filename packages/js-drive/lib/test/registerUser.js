const BitcoreLib = require('@dashevo/dashcore-lib');

const { PrivateKey } = BitcoreLib;
const { Registration } = BitcoreLib.Transaction.SubscriptionTransactions;

/**
 * Register user
 * @param {string} username
 * @param {RpcClient} api
 * @returns {Promise<object>}
 */
async function registerUser(username, api) {
  let response = await api.getnewaddress();
  const address = response.result;

  response = await api.dumpprivkey(address);
  const privateKeyString = response.result;

  const privateKey = new PrivateKey(privateKeyString);

  await api.generate(101);
  await api.sendtoaddress(address, 10);
  await api.generate(7);

  response = await api.listunspent();
  const unspent = response.result;
  const inputs = unspent.filter(input => input.address === address);

  const subTx = Registration
    .createRegistration(username, privateKey)
    .fund(inputs, address, 100000)
    .sign(privateKey)
    .serialize();

  response = await api.sendrawtransaction(subTx);

  return {
    userId: response.result,
    privateKeyString,
    address,
  };
}

module.exports = registerUser;
