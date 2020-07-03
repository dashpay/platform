module.exports = async function getStatus() {
  const { height, relayFee, network } = this;
  return {
    coreVersion: 150000,
    protocolVersion: 70216,
    blocks: height,
    timeOffset: 0,
    proxy: '',
    difficulty: null,
    testnet: false,
    relayFee,
    errors: '',
    network,
  };
};
