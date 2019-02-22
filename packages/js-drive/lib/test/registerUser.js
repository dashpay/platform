const BitcoreLib = require('@dashevo/dashcore-lib');

const { PrivateKey, Transaction } = BitcoreLib;

/**
 * Register user
 * @param {string} userName
 * @param {RpcClient} api
 * @returns {Promise<{ userId: string, privateKeyString: string, address: string }>}
 */
async function registerUser(userName, api) {
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

  const transactionPayload = new Transaction.Payload.SubTxRegisterPayload();

  transactionPayload.setUserName(userName)
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

  const { result: userId } = await api.sendrawtransaction(transaction.serialize());

  return {
    userId,
    privateKeyString,
    address,
  };
}

module.exports = registerUser;
