const CONSTANTS = {
  BIP45: 'BIP45',
  BIP44: 'BIP44',
  DUFFS_PER_DASH: 100000000,
  BIP44_ADDRESS_GAP: 20,
  // TODO : When chainlock is launched in mainnet, reduce this to 1 \0/
  SECURE_TRANSACTION_CONFIRMATIONS_NB: 6,
  BIP32__ROOT_PATH: 'm',
  // Livenet is 5 for Dash.
  BIP44_LIVENET_ROOT_PATH: "m/44'/5'",
  // All testnet coins are 1's
  BIP44_TESTNET_ROOT_PATH: "m/44'/1'",

  // Livenet is 5 for Dash.
  DIP9_LIVENET_ROOT_PATH: "m/9'/5'",
  // All testnet coins are 1's
  DIP9_TESTNET_ROOT_PATH: "m/9'/1'",
  // The max amount of an UTXO to be considered too big to be used in the tx before exploring
  // smaller alternatives (proportinal to tx amount).
  UTXO_SELECTION_MAX_SINGLE_UTXO_FACTOR: 2,
  // The minimum amount an UTXO need to contribute proportional to tx amount.
  UTXO_SELECTION_MIN_TX_AMOUNT_VS_UTXO_FACTOR: 0.1,
  // The maximum threshold to consider fees non-significant in relation to tx amount.
  UTXO_SELECTION_MAX_FEE_VS_TX_AMOUNT_FACTOR: 0.05,
  // The maximum amount to pay for using small inputs instead of one big input
  // when fees are significant (proportional to how much we would pay for using that big input only)
  UTXO_SELECTION_MAX_FEE_VS_SINGLE_UTXO_FEE_FACTOR: 5,
  MAX_STANDARD_TX_SIZE: 100000,
  MAX_P2SH_SIGOPS: 15,
  COINBASE_MATURITY: 100,
  // limit to how many times an unconfirmed input in a new tx can be respent
  UTXO_CHAINED_SPENDING_LIMIT_FOR_TX: 25,
  FEES: {
    DUST_RELAY_TX_FEE: 1000,
    ZERO: 0,
    ECONOMIC: 500,
    NORMAL: 1000,
    PRIORITY: 10000,
    // Fee for IS are 0.0001 * INPUTS
    INSTANT_FEE_PER_INPUTS: 10000,
  },
  UNCONFIRMED_TRANSACTION_STATUS_CODE: -1,
  WALLET_TYPES: {
    ADDRESS: 'address',
    PUBLICKEY: 'publicKey',
    PRIVATEKEY: 'privateKey',
    // TODO: DEPRECATE.
    SINGLE_ADDRESS: 'single_address',
    // TODO: DEPRECATE.
    HDWALLET: 'hdwallet',
    HDPRIVATE: 'hdprivate',
    HDPUBLIC: 'hdpublic',
  },
  // List of account function and properties that can be injected in a plugin
  INJECTION_LISTS: {
    SAFE_FUNCTIONS: [
      'createTransaction',
      'createTransactionFromUTXOS',
      'getUTXOS',
      'getUnusedAddress',
      'getConfirmedBalance',
      'getUnconfirmedBalance',
      'getTotalBalance',
      'broadcastTransaction',
      'importTransactions',
      'importBlockHeader',
      'getAddress',
      'fetchStatus',
      'getPlugin',
      'sign',
      'getTransactions',
      'getTransactionHistory',
      'forceRefreshAccount',
      'disconnect',
      'connect',
    ],
    UNSAFE_FUNCTIONS: [
      'generateAddress',
      'getPrivateKeys',
      'injectPlugin',
    ],
    UNSAFE_PROPERTIES: [
      'storage',
      'identities',
    ],
    SAFE_PROPERTIES: [
      'offlineMode',
      'index',
      'BIP44PATH',
      'transport',
      'walletId',
      'walletType',
      'strategy',
      'network',
    ],
  },
  TRANSACTION_HISTORY_TYPES: {
    RECEIVED: 'received',
    SENT: 'sent',
    ADDRESS_TRANSFER: 'address_transfer',
    ACCOUNT_TRANSFER: 'account_transfer',
    UNKNOWN: 'unknown',
  },
  STORAGE: {
    version: 2,
    autosaveIntervalTime: 10 * 1000,
  },
  TXIN_OUTPOINT_TXID_BYTES: 36,
  TXIN_OUTPOINT_INDEX_BYTES: 4,
  TXIN_SEQUENCE_BYTES: 4,
  TXOUT_DUFFS_VALUE_BYTES: 8,
  VERSION_BYTES: 4,
  N_LOCKTIME_BYTES: 4,

  BLOOM_FALSE_POSITIVE_RATE: 0.0001,
  NULL_HASH: '0000000000000000000000000000000000000000000000000000000000000000',
};
module.exports = CONSTANTS;
