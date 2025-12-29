/**
 * Requirements for wasm-sdk functional tests.
 * These IDs/contracts should exist on the target network
 * (seeded via SDK_TEST_DATA=true yarn start).
 * @returns {object} Test requirements object
 */
// eslint-disable-next-line import/prefer-default-export
export function wasmFunctionalTestRequirements() {
  return {
    // Seeded via SDK_TEST_DATA=true (identity id = 32 bytes of 0x01)
    identityId: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi',
    dpnsContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    dpnsDomain: {
      // The 'dash' TLD exists by default on any network
      parent: '',
      label: 'dash',
    },
    tokenContracts: [
      // Seeded token contract (contract id = 32 bytes of 0x03)
      { contractId: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8', position: 0 },
    ],
    // Local-only data: evonode ProTx / epoch numbers are not seeded by SDK_TEST_DATA
    evonodeProTxHash: null,
    sampleEpoch: null,
  };
}
