/**
 * Send ST packet and it's Transaction to Drive and Core
 *
 * @param {STPacket} stPacket
 * @param {Transaction} stateTransition
 * @param {RpcClient} driveClient
 * @param {RpcClient} coreClient
 *
 * @returns {Promise<string>}
 */
async function sendSTPacket(stPacket, stateTransition, driveClient, coreClient) {
  const params = {
    stPacket: stPacket.serialize().toString('hex'),
    stateTransition: stateTransition.serialize(),
  };

  const { error } = await driveClient.request('addSTPacket', params);

  if (error) {
    const e = new Error();
    e.originalError = error;
    throw e;
  }

  const { result: tsId } = await coreClient.sendRawTransaction(stateTransition);

  await coreClient.generate(1);

  return tsId;
}

module.exports = sendSTPacket;
