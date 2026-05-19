import Dashcore from '@dashevo/dashcore-lib';
import {
  BIP44_LIVENET_ROOT_PATH, BIP44_TESTNET_ROOT_PATH,
} from '../../CONSTANTS.js';
/**
 * Will return a root account path
 * @param network - default : 'testnet'
 * @param accountIndex - default : 0
 * @return {string} - BIP44 Path to account
 */
export default function getBIP44Path(network, accountIndex = 0) {
  return (network === Dashcore.Networks.livenet.toString())
    ? `${BIP44_LIVENET_ROOT_PATH}/${accountIndex}'`
    : `${BIP44_TESTNET_ROOT_PATH}/${accountIndex}'`;
};
