const { PrivateKey, Transaction } = require('@dashevo/dashcore-lib');

const User = require('./User');

/**
 * Register single blockchain user
 *
 * @param {string} username
 * @param {RpcClient} rpcClient
 *
 * @returns {Promise<User>}
 */
async function registerUser(username, rpcClient) {
  let response = await rpcClient.getnewaddress();
  const address = response.result;

  response = await rpcClient.dumpprivkey(address);
  const privateKeyString = response.result;

  const privateKey = new PrivateKey(privateKeyString);

  await rpcClient.generate(101);
  await rpcClient.sendtoaddress(address, 10);
  await rpcClient.generate(7);

  response = await rpcClient.listunspent();
  const unspent = response.result;
  const inputs = unspent.filter(input => input.address === address);

  const transactionPayload = new Transaction.Payload.SubTxRegisterPayload();

  transactionPayload.setUserName(username)
    .setPubKeyIdFromPrivateKey(privateKey)
    .sign(privateKey);

  const transaction = new Transaction({
    type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
    version: 3,
    extraPayload: transactionPayload.toString(),
  });

  transaction.from(inputs)
    .addFundingOutput(10000)
    .change(address)
    .fee(668)
    .sign(privateKey);

  const { result: userId } = await rpcClient.sendrawtransaction(transaction.serialize());

  return new User(userId, privateKey);
}

module.exports = registerUser;
