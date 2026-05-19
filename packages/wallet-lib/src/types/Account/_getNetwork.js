import Dashcore from '@dashevo/dashcore-lib';

export default function getNetwork(network) {
  return Dashcore.Networks[network].toString() || Dashcore.Networks.testnet.toString();
};
