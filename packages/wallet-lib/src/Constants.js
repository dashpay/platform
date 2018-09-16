const Constants = {
  BIP45: 'BIP45',
  BIP44: 'BIP44',
  DUFFS_PER_DASH: 100000000,
  BIP44_ADDRESS_GAP: 20,
  BIP32__ROOT_PATH: 'm',
  // Livenet is 5 for Dash.
  BIP44_LIVENET_ROOT_PATH: "m/44'/5'",
  // All testnet coins are 1's
  BIP44_TESTNET_ROOT_PATH: "m/44'/1'",
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
  UTXO_MAX_INOPUTS_PER_TX: 25,
  FEES: {
    // Fee for IS are 0.0001 * INPUTS
    INSTANT_FEE_PER_INPUTS: 10000,
    // Todo : Ensure value.
    // Need to be multiplied by a multiplier (*1.5 / *2) to higher the priority
    FEE_PER_KB: 1000,
  },
};
module.exports = Constants;
