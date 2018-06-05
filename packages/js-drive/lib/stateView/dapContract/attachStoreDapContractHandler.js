/**
 * Store DAP contract handler
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {storeDapContract} storeDapContract
 */
function attachStoreDapContractHandler(stHeadersReader, storeDapContract) {
  stHeadersReader.on('header', async (header) => {
    const cid = header.getPacketCID();
    await storeDapContract(cid);
  });
}

module.exports = attachStoreDapContractHandler;
