// TS basic exports
import { // eslint-disable-next-line
  Client, Platform, Core, Essentials, Wallet, WalletLib
} from '..';

// Test Client constructor options
// eslint-disable-next-line
new Client({
  wallet: {
    mnemonic: 'test',
  },
  network: 'testnet',
  dapiAddresses: [],
  dapiAddressProvider: null,
  seeds: [],
  timeout: 1000,
  retries: 5,
  baseBanTime: 1000,
  blockHeadersProviderOptions: {},
  blockHeadersProvider: {},
  driveProtocolVersion: 1,
  apps: {},
});
